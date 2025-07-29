use crate::json::Encoding;
use jomini::{text::ObjectReader, Scalar, TextTape, TextToken, Utf8Encoding, Windows1252Encoding};
use std::collections::{HashMap, HashSet};

pub struct InterpolatedTape<'a> {
    original_tape: &'a TextTape<'a>,
    interpolated_strings: Vec<String>,
    token_overrides: HashMap<usize, usize>, // token_index -> string_index
    variable_declarations: HashSet<String>, // variable names that were declared
}

/// Decode bytes using the specified encoding
fn decode_bytes(bytes: &[u8], encoding: Encoding) -> Result<String, Box<dyn std::error::Error>> {
    let decoded = match encoding {
        Encoding::Utf8 => Utf8Encoding::decode(bytes),
        Encoding::Windows1252 => Windows1252Encoding::decode(bytes),
    };
    Ok(decoded.into_owned())
}

impl<'a> InterpolatedTape<'a> {
    /// Create a new InterpolatedTape from an existing tape with interpolations applied
    /// Uses the specified encoding to decode text tokens
    pub fn from_tape_with_interpolation(
        tape: &'a TextTape<'a>,
        encoding: Encoding,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut variables: HashMap<String, f64> = HashMap::new();
        let mut interpolated_strings = Vec::new();
        let mut token_overrides = HashMap::new();
        let mut skip_interpolation: HashSet<usize> = HashSet::new();
        let mut variable_declarations: HashSet<String> = HashSet::new();

        // Multiple passes to collect variable definitions (handle forward references)
        let tokens = tape.tokens();
        let mut unresolved_refs: Vec<(usize, String, String)> = Vec::new(); // (index, var_name, referenced_var)

        // Keep iterating until no more variables can be resolved
        let mut progress = true;
        while progress {
            progress = false;
            let mut i = 0;

            while i < tokens.len() {
                if let TextToken::Unquoted(scalar) = &tokens[i] {
                    let text = decode_bytes(scalar.as_bytes(), encoding)?;

                    // Variable definition: @var_name
                    if text.starts_with('@') && !text.starts_with("@[") {
                        let var_name = &text[1..];

                        // Skip if already processed
                        if variables.contains_key(var_name) {
                            i += 1;
                            continue;
                        }

                        // Look for the value (no = operator in tokens, it's consumed by parser)
                        if i + 1 < tokens.len() {
                            if let TextToken::Unquoted(value_scalar) = &tokens[i + 1] {
                                let value_text = decode_bytes(value_scalar.as_bytes(), encoding)?;

                                // Handle @var = @[expression] format
                                if value_text.starts_with("@[") && value_text.ends_with("]") {
                                    let expr = &value_text[2..value_text.len() - 1];
                                    let computed_value = eval_expression(expr, &variables)?;
                                    variables.insert(var_name.to_string(), computed_value);
                                    skip_interpolation.insert(i);
                                    // Mark this as a variable declaration
                                    variable_declarations.insert(format!("@{}", var_name));
                                    progress = true;
                                }
                                // Handle @var = @other_var format (direct variable reference)
                                else if value_text.starts_with("@")
                                    && !value_text.starts_with("@[")
                                {
                                    let referenced_var = &value_text[1..];
                                    if let Some(&referenced_value) = variables.get(referenced_var) {
                                        variables.insert(var_name.to_string(), referenced_value);
                                        skip_interpolation.insert(i);
                                        // Mark this as a variable declaration
                                        variable_declarations.insert(format!("@{}", var_name));
                                        progress = true;
                                    } else {
                                        // Store for later resolution
                                        unresolved_refs.push((
                                            i,
                                            var_name.to_string(),
                                            referenced_var.to_string(),
                                        ));
                                    }
                                }
                                // Handle @var = number format
                                else if let Ok(value) = parse_f64(value_scalar.as_bytes()) {
                                    variables.insert(var_name.to_string(), value);
                                    skip_interpolation.insert(i);
                                    // Mark this as a variable declaration
                                    variable_declarations.insert(format!("@{}", var_name));
                                    progress = true;
                                }
                            }
                        }
                    }
                }
                i += 1;
            }

            // Try to resolve any unresolved references
            let mut resolved_indices = Vec::new();
            for (idx, (token_index, var_name, referenced_var)) in unresolved_refs.iter().enumerate()
            {
                if let Some(&referenced_value) = variables.get(referenced_var) {
                    variables.insert(var_name.clone(), referenced_value);
                    skip_interpolation.insert(*token_index);
                    variable_declarations.insert(format!("@{}", var_name));
                    resolved_indices.push(idx);
                    progress = true;
                }
            }

            // Remove resolved references from the unresolved list
            for &idx in resolved_indices.iter().rev() {
                unresolved_refs.remove(idx);
            }
        }

        // Check for any remaining unresolved references
        if !unresolved_refs.is_empty() {
            let unresolved_names: Vec<String> = unresolved_refs
                .iter()
                .map(|(_, var_name, referenced_var)| {
                    format!("@{} -> @{}", var_name, referenced_var)
                })
                .collect();
            return Err(format!(
                "Unresolved variable references: {}",
                unresolved_names.join(", ")
            )
            .into());
        }

        // Second pass: find and store interpolations
        let mut i = 0;
        while i < tokens.len() {
            if let TextToken::Unquoted(scalar) = &tokens[i] {
                let text = decode_bytes(scalar.as_bytes(), encoding)?;

                // Variable interpolation: @[expression] or @var_name
                if text.starts_with("@[") && text.ends_with("]") {
                    let expr = &text[2..text.len() - 1];
                    let computed_value = eval_expression(expr, &variables)?;
                    let value_str = format_numeric_value(computed_value);
                    let string_index = interpolated_strings.len();
                    interpolated_strings.push(value_str);
                    token_overrides.insert(i, string_index);
                } else if text.starts_with("@")
                    && !text.starts_with("@[")
                    && !skip_interpolation.contains(&i)
                {
                    let var_name = &text[1..];
                    if let Some(&value) = variables.get(var_name) {
                        let value_str = format_numeric_value(value);
                        let string_index = interpolated_strings.len();
                        interpolated_strings.push(value_str);
                        token_overrides.insert(i, string_index);
                    }
                }
            }
            i += 1;
        }

        Ok(Self {
            original_tape: tape,
            interpolated_strings,
            token_overrides,
            variable_declarations,
        })
    }

    /// Materialize all tokens into a tape that owns its string data
    /// This allows using the full jomini API (JSON, readers, etc.)
    pub fn materialize(&self) -> MaterializedTape {
        let mut owned_strings = Vec::new();
        let mut string_to_index: HashMap<String, usize> = HashMap::new();
        let mut token_data = Vec::new();

        // Helper function to get or create string index
        let mut get_string_index = |s: String| -> usize {
            if let Some(&existing_index) = string_to_index.get(&s) {
                existing_index
            } else {
                let index = owned_strings.len();
                string_to_index.insert(s.clone(), index);
                owned_strings.push(s);
                index
            }
        };

        // Process all tokens
        for i in 0..self.original_tape.tokens().len() {
            let original_token = &self.original_tape.tokens()[i];

            // Check if this token has an interpolated override
            if let Some(&string_index) = self.token_overrides.get(&i) {
                // Use interpolated value - force it to be Unquoted
                let interpolated_str = &self.interpolated_strings[string_index];
                let owned_string_index = get_string_index(interpolated_str.clone());
                token_data.push((TokenType::Unquoted, Some(owned_string_index)));
            } else {
                // Use original token
                match original_token {
                    TextToken::Unquoted(scalar) => {
                        let text = String::from_utf8_lossy(scalar.as_bytes()).into_owned();
                        let owned_string_index = get_string_index(text);
                        token_data.push((TokenType::Unquoted, Some(owned_string_index)));
                    }
                    TextToken::Quoted(scalar) => {
                        let text = String::from_utf8_lossy(scalar.as_bytes()).into_owned();
                        let owned_string_index = get_string_index(text);
                        token_data.push((TokenType::Quoted, Some(owned_string_index)));
                    }
                    TextToken::Header(scalar) => {
                        let text = String::from_utf8_lossy(scalar.as_bytes()).into_owned();
                        let owned_string_index = get_string_index(text);
                        token_data.push((TokenType::Header, Some(owned_string_index)));
                    }
                    TextToken::Parameter(scalar) => {
                        let text = String::from_utf8_lossy(scalar.as_bytes()).into_owned();
                        let owned_string_index = get_string_index(text);
                        token_data.push((TokenType::Parameter, Some(owned_string_index)));
                    }
                    TextToken::UndefinedParameter(scalar) => {
                        let text = String::from_utf8_lossy(scalar.as_bytes()).into_owned();
                        let owned_string_index = get_string_index(text);
                        token_data.push((TokenType::UndefinedParameter, Some(owned_string_index)));
                    }
                    TextToken::Array { end, mixed } => {
                        token_data.push((
                            TokenType::Array {
                                end: *end,
                                mixed: *mixed,
                            },
                            None,
                        ));
                    }
                    TextToken::Object { end, mixed } => {
                        token_data.push((
                            TokenType::Object {
                                end: *end,
                                mixed: *mixed,
                            },
                            None,
                        ));
                    }
                    TextToken::Operator(op) => {
                        token_data.push((TokenType::Operator(*op), None));
                    }
                    TextToken::End(end) => {
                        token_data.push((TokenType::End(*end), None));
                    }
                    TextToken::MixedContainer => {
                        token_data.push((TokenType::MixedContainer, None));
                    }
                }
            }
        }

        MaterializedTape {
            owned_strings,
            token_data,
        }
    }

    /// Write JSON output with filtering directly to a writer with options
    pub fn to_writer_with_options<W: std::io::Write>(
        &self,
        writer: W,
        options: jomini::json::JsonOptions,
        encoding: Encoding,
    ) -> std::io::Result<()> {
        // Create filtered tokens that exclude variable declarations
        let materialized = self.materialize();
        let filtered_tokens = materialized.create_filtered_tokens(&self.variable_declarations);

        // Use jomini's built-in JSON serialization with proper options
        match encoding {
            Encoding::Utf8 => {
                let reader = ObjectReader::from_tokens(&filtered_tokens, Utf8Encoding::new());
                reader.json().with_options(options).to_writer(writer)
            }
            Encoding::Windows1252 => {
                let reader =
                    ObjectReader::from_tokens(&filtered_tokens, Windows1252Encoding::new());
                reader.json().with_options(options).to_writer(writer)
            }
        }
    }
}

/// A materialized tape that owns all string data and provides token access
pub struct MaterializedTape {
    /// Owns all string data (both original and interpolated)
    owned_strings: Vec<String>,
    /// Token data without string references - we recreate tokens on demand
    token_data: Vec<(TokenType, Option<usize>)>, // (token_type, string_index)
}

#[derive(Clone, Debug)]
enum TokenType {
    Unquoted,
    Quoted,
    Header,
    Parameter,
    UndefinedParameter,
    Array { end: usize, mixed: bool },
    Object { end: usize, mixed: bool },
    Operator(jomini::text::Operator),
    End(usize),
    MixedContainer,
}

impl MaterializedTape {
    /// Create tokens referencing our owned string data
    pub fn create_tokens(&self) -> Vec<TextToken<'_>> {
        let mut tokens = Vec::new();

        for (token_type, string_index_opt) in &self.token_data {
            let token = match (token_type, string_index_opt) {
                (TokenType::Unquoted, Some(idx)) => {
                    TextToken::Unquoted(Scalar::new(self.owned_strings[*idx].as_bytes()))
                }
                (TokenType::Quoted, Some(idx)) => {
                    TextToken::Quoted(Scalar::new(self.owned_strings[*idx].as_bytes()))
                }
                (TokenType::Header, Some(idx)) => {
                    TextToken::Header(Scalar::new(self.owned_strings[*idx].as_bytes()))
                }
                (TokenType::Parameter, Some(idx)) => {
                    TextToken::Parameter(Scalar::new(self.owned_strings[*idx].as_bytes()))
                }
                (TokenType::UndefinedParameter, Some(idx)) => {
                    TextToken::UndefinedParameter(Scalar::new(self.owned_strings[*idx].as_bytes()))
                }
                (TokenType::Array { end, mixed }, None) => TextToken::Array {
                    end: *end,
                    mixed: *mixed,
                },
                (TokenType::Object { end, mixed }, None) => TextToken::Object {
                    end: *end,
                    mixed: *mixed,
                },
                (TokenType::Operator(op), None) => TextToken::Operator(*op),
                (TokenType::End(end), None) => TextToken::End(*end),
                (TokenType::MixedContainer, None) => TextToken::MixedContainer,
                _ => panic!("Invalid token type/string index combination"),
            };
            tokens.push(token);
        }

        tokens
    }

    /// Create filtered tokens that exclude variable declarations
    pub fn create_filtered_tokens(
        &self,
        variable_declarations: &std::collections::HashSet<String>,
    ) -> Vec<TextToken<'_>> {
        let original_tokens = self.create_tokens();
        TokenFilter::filter_tokens_static(&original_tokens, variable_declarations)
    }
}

/// Token filter that maintains stream integrity while removing variable declarations
struct TokenFilter;

impl TokenFilter {
    /// Filter tokens to remove variable declarations while maintaining token stream integrity
    fn filter_tokens_static<'a>(
        tokens: &[TextToken<'a>],
        variable_declarations: &std::collections::HashSet<String>,
    ) -> Vec<TextToken<'a>> {
        let mut filtered_tokens = Vec::new();
        let mut index_mapping = std::collections::HashMap::new(); // original_index -> filtered_index

        // First pass: collect all indices that should be kept
        let mut i = 0;
        while i < tokens.len() {
            let should_skip = Self::should_skip_token_sequence(tokens, i, variable_declarations);

            if should_skip.skip {
                // Skip the entire sequence (key + operator + value)
                i = should_skip.next_index;
                continue;
            }

            // Map original index to filtered index
            index_mapping.insert(i, filtered_tokens.len());

            // Convert EXACT and EXISTS operators to regular assignment
            let mut token = tokens[i].clone();
            if let TextToken::Operator(op) = &token {
                if matches!(
                    op,
                    jomini::text::Operator::Exact | jomini::text::Operator::Exists
                ) {
                    token = TextToken::Operator(jomini::text::Operator::Equal);
                }
            }

            filtered_tokens.push(token);
            i += 1;
        }

        // Second pass: update all token indices based on the mapping
        for token in &mut filtered_tokens {
            *token = Self::update_token_indices(token, &index_mapping);
        }

        filtered_tokens
    }

    /// Check if we should skip a token sequence starting at the given index
    fn should_skip_token_sequence(
        tokens: &[TextToken<'_>],
        start_index: usize,
        variable_declarations: &std::collections::HashSet<String>,
    ) -> SkipResult {
        if let Some(TextToken::Unquoted(scalar)) = tokens.get(start_index) {
            let text = String::from_utf8_lossy(scalar.as_bytes());

            if variable_declarations.contains(&*text) {
                // This is a variable declaration - calculate how many tokens to skip
                let mut skip_to = start_index + 1; // Skip the key

                // Skip operator if present
                if let Some(TextToken::Operator(_)) = tokens.get(skip_to) {
                    skip_to += 1;
                }

                // Skip the value
                if let Some(value_token) = tokens.get(skip_to) {
                    skip_to += 1;

                    // If the value is a container, skip to its end
                    match value_token {
                        TextToken::Object { end, .. } | TextToken::Array { end, .. } => {
                            skip_to = *end + 1;
                        }
                        _ => {} // Simple value, already incremented
                    }
                }

                return SkipResult {
                    skip: true,
                    next_index: skip_to,
                };
            }
        }

        SkipResult {
            skip: false,
            next_index: start_index + 1,
        }
    }

    /// Update token indices for containers to maintain token stream integrity
    fn update_token_indices<'a>(
        token: &TextToken<'a>,
        index_mapping: &std::collections::HashMap<usize, usize>,
    ) -> TextToken<'a> {
        match token {
            TextToken::Object { end, mixed } => {
                let new_end = index_mapping.get(end).copied().unwrap_or(*end);
                TextToken::Object {
                    end: new_end,
                    mixed: *mixed,
                }
            }
            TextToken::Array { end, mixed } => {
                let new_end = index_mapping.get(end).copied().unwrap_or(*end);
                TextToken::Array {
                    end: new_end,
                    mixed: *mixed,
                }
            }
            TextToken::End(end) => {
                let new_end = index_mapping.get(end).copied().unwrap_or(*end);
                TextToken::End(new_end)
            }
            _ => token.clone(),
        }
    }
}

struct SkipResult {
    skip: bool,
    next_index: usize,
}

/// Format a numeric value as a string
fn format_numeric_value(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{}", value)
    }
}

fn parse_f64(s: &[u8]) -> Result<f64, Box<dyn std::error::Error>> {
    let scalar = Scalar::new(s);
    scalar.to_f64().map_err(|e| e.into())
}

fn eval_expression(
    expr: &str,
    variables: &HashMap<String, f64>,
) -> Result<f64, Box<dyn std::error::Error>> {
    // Enhanced expression evaluator with proper parentheses and operator precedence
    // Handles: numbers, variables, +, -, *, /, parentheses with proper precedence
    let expr = expr.trim();

    // Remove outer brackets if present
    let expr = if expr.starts_with('[') && expr.ends_with(']') {
        &expr[1..expr.len() - 1]
    } else {
        expr
    };

    eval_addition_subtraction(expr, variables)
}

fn contains_additive_ops_at_depth_zero(s: &str) -> bool {
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            '+' | '-' if depth == 0 => return true,
            _ => {}
        }
    }
    false
}

fn eval_addition_subtraction(
    expr: &str,
    variables: &HashMap<String, f64>,
) -> Result<f64, Box<dyn std::error::Error>> {
    // Handle + and - operations (lowest precedence)
    // For left-associativity with recursive descent, find the RIGHTMOST operator
    let expr = expr.trim();

    // Handle negative numbers at the start (but only if remainder has no +/- at depth 0)
    if expr.starts_with('-')
        && !expr[1..].starts_with('(')
        && !contains_additive_ops_at_depth_zero(&expr[1..])
    {
        let operand = &expr[1..];
        return Ok(-eval_multiplication_division(operand, variables)?);
    }

    // Find + or - operators that are not inside parentheses, scanning right to left
    let mut paren_depth = 0;
    let chars: Vec<char> = expr.chars().collect();

    for i in (0..chars.len()).rev() {
        match chars[i] {
            ')' => paren_depth += 1,
            '(' => paren_depth -= 1,
            '+' | '-' if paren_depth == 0 && i > 0 => {
                let left_part = &expr[..i].trim();
                let right_part = &expr[i + 1..].trim();
                let left_val = eval_addition_subtraction(left_part, variables)?;
                let right_val = eval_multiplication_division(right_part, variables)?;

                return Ok(if chars[i] == '+' {
                    left_val + right_val
                } else {
                    left_val - right_val
                });
            }
            _ => {}
        }
    }

    eval_multiplication_division(expr, variables)
}

fn eval_multiplication_division(
    expr: &str,
    variables: &HashMap<String, f64>,
) -> Result<f64, Box<dyn std::error::Error>> {
    // Handle * and / operations (higher precedence)
    // For left-associativity with recursive descent, find the RIGHTMOST operator
    let expr = expr.trim();

    // Find * or / operators that are not inside parentheses, scanning right to left
    let mut paren_depth = 0;
    let chars: Vec<char> = expr.chars().collect();

    for i in (0..chars.len()).rev() {
        match chars[i] {
            ')' => paren_depth += 1,
            '(' => paren_depth -= 1,
            '*' | '/' if paren_depth == 0 && i > 0 => {
                let left_part = &expr[..i].trim();
                let right_part = &expr[i + 1..].trim();
                let left_val = eval_multiplication_division(left_part, variables)?;
                let right_val = eval_factor(right_part, variables)?;

                return Ok(if chars[i] == '*' {
                    left_val * right_val
                } else {
                    left_val / right_val
                });
            }
            _ => {}
        }
    }

    eval_factor(expr, variables)
}

fn eval_factor(
    expr: &str,
    variables: &HashMap<String, f64>,
) -> Result<f64, Box<dyn std::error::Error>> {
    // Handle parentheses and basic operands (highest precedence)
    let expr = expr.trim();

    // Handle parenthesized expressions
    if expr.starts_with('(') && expr.ends_with(')') {
        let inner = &expr[1..expr.len() - 1];
        return eval_addition_subtraction(inner, variables);
    }

    // Handle negative expressions with parentheses
    if expr.starts_with('-') && expr[1..].starts_with('(') && expr.ends_with(')') {
        let inner = &expr[2..expr.len() - 1];
        return Ok(-eval_addition_subtraction(inner, variables)?);
    }

    eval_simple_operand(expr, variables)
}

fn eval_simple_operand(
    operand: &str,
    variables: &HashMap<String, f64>,
) -> Result<f64, Box<dyn std::error::Error>> {
    let operand = operand.trim();

    if let Some(&value) = variables.get(operand) {
        return Ok(value);
    }

    if let Ok(num) = operand.parse::<f64>() {
        return Ok(num);
    }

    // Handle negative variables
    if let Some(var_name) = operand.strip_prefix('-') {
        if let Some(&value) = variables.get(var_name) {
            return Ok(-value);
        }
    }

    Err(format!("Unknown operand: {}", operand).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    impl<'a> InterpolatedTape<'a> {
        /// Generate JSON output using default options (UTF-8 encoding for tests)
        pub fn to_json(&self) -> String {
            let default_options = jomini::json::JsonOptions::new();
            let mut output = Vec::new();
            self.to_writer_with_options(&mut output, default_options, Encoding::Utf8)
                .unwrap();
            String::from_utf8(output).unwrap()
        }
    }

    #[test]
    fn test_basic_variable_interpolation() -> Result<(), Box<dyn std::error::Error>> {
        let data = br#"
@my_var = 10
my_obj = {
  pos_x = @[100-my_var]
  pos_y = @my_var
}
"#;

        let tape = TextTape::from_slice(data)?;
        let interpolated_tape =
            InterpolatedTape::from_tape_with_interpolation(&tape, Encoding::Utf8)?;

        let json_output = interpolated_tape.to_json();
        let expected_json = r#"{"my_obj":{"pos_x":90,"pos_y":10}}"#;
        assert_eq!(json_output, expected_json);

        Ok(())
    }

    #[test]
    fn test_arithmetic_expressions() -> Result<(), Box<dyn std::error::Error>> {
        let data = br#"
@half = @[1/2]
scale = @[1-half]
scale_mul = @[1*half]
scale_add = @[1+half]
scale_div = @[1/half]
my_list = { @[1-half] @half }
"#;

        let tape = TextTape::from_slice(data)?;
        let interpolated_tape =
            InterpolatedTape::from_tape_with_interpolation(&tape, Encoding::Utf8)?;

        let json_output = interpolated_tape.to_json();
        let expected_json =
            r#"{"scale":0.5,"scale_mul":0.5,"scale_add":1.5,"scale_div":2,"my_list":[0.5,0.5]}"#;
        assert_eq!(json_output, expected_json);

        Ok(())
    }

    #[test]
    fn test_complex_expressions() -> Result<(), Box<dyn std::error::Error>> {
        let data = br#"@half = @[1/2]
my_calc = @[(-half-half)*half]"#;

        let tape = TextTape::from_slice(data)?;
        let interpolated_tape =
            InterpolatedTape::from_tape_with_interpolation(&tape, Encoding::Utf8)?;

        let json_output = interpolated_tape.to_json();
        let expected_json = r#"{"my_calc":-0.5}"#;
        assert_eq!(json_output, expected_json);

        Ok(())
    }

    #[test]
    fn test_interpolation_equivalence() -> Result<(), Box<dyn std::error::Error>> {
        let interpolated_data = br#"
@my_var = 10
my_obj = {
  pos_x = @[100-my_var]
  pos_y = @my_var
}
"#;

        let interpolated_tape_raw = TextTape::from_slice(interpolated_data)?;
        let interpolated_tape =
            InterpolatedTape::from_tape_with_interpolation(&interpolated_tape_raw, Encoding::Utf8)?;

        // Assert against complete JSON output
        let json_output = interpolated_tape.to_json();
        let expected_json = r#"{"my_obj":{"pos_x":90,"pos_y":10}}"#;
        assert_eq!(json_output, expected_json);

        Ok(())
    }

    #[test]
    fn test_nested_variable_filtering() -> Result<(), Box<dyn std::error::Error>> {
        let data = br#"
obj = { @half = 0.5 pos_x=@half pos_y=@[half*2] }
scale = @[1-0.25]
"#;

        let tape = TextTape::from_slice(data)?;
        let interpolated_tape =
            InterpolatedTape::from_tape_with_interpolation(&tape, Encoding::Utf8)?;

        let json_output = interpolated_tape.to_json();
        let expected_json = r#"{"obj":{"pos_x":0.5,"pos_y":1},"scale":0.75}"#;
        assert_eq!(json_output, expected_json);

        Ok(())
    }

    #[test]
    fn test_comprehensive_interpolation_with_complete_json(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let data = br#"@base = 10
@factor = @[1/4]
@doubled = @[base*2]
@halved = @[base/2]
width = @doubled
height = @base
ratio = @factor"#;

        let tape = TextTape::from_slice(data)?;
        let interpolated_tape =
            InterpolatedTape::from_tape_with_interpolation(&tape, Encoding::Utf8)?;

        let json_output = interpolated_tape.to_json();

        // Assert against complete JSON output
        let expected_json = r#"{"width":20,"height":10,"ratio":0.25}"#;
        assert_eq!(json_output, expected_json);

        Ok(())
    }

    #[test]
    fn test_parentheses_interpolation() -> Result<(), Box<dyn std::error::Error>> {
        let data = br#"
@width = 768
@cross_x = @[ ( 333 / width ) + 0.001 ]
test_value = @cross_x
"#;

        let tape = TextTape::from_slice(data)?;
        let interpolated_tape =
            InterpolatedTape::from_tape_with_interpolation(&tape, Encoding::Utf8)?;

        let json_output = interpolated_tape.to_json();
        let expected_json = r#"{"test_value":0.43459375}"#;
        assert_eq!(json_output, expected_json);

        Ok(())
    }

    #[test]
    fn test_chained_divisions() -> Result<(), Box<dyn std::error::Error>> {
        let data = br#"
@test1 = @[1/3/2]
@test2 = @[8/4/2]
@test3 = @[12/3/2/2]
result1 = @test1
result2 = @test2
result3 = @test3
"#;

        let tape = TextTape::from_slice(data)?;
        let interpolated_tape =
            InterpolatedTape::from_tape_with_interpolation(&tape, Encoding::Utf8)?;

        let json_output = interpolated_tape.to_json();
        let expected_json = r#"{"result1":0.16666666666666666,"result2":1,"result3":1}"#;
        assert_eq!(json_output, expected_json);

        Ok(())
    }

    #[test]
    fn test_unary_minus_with_addition() -> Result<(), Box<dyn std::error::Error>> {
        let data = br#"
@half = @[1/2]
@two = @[2]
test1 = @[-half+2]
test2 = @[-half + half]
test3 = @[-2+half]
test4 = @[-half*2+1]
test5 = @[-(half)+2]
"#;

        let tape = TextTape::from_slice(data)?;
        let interpolated_tape =
            InterpolatedTape::from_tape_with_interpolation(&tape, Encoding::Utf8)?;

        let json_output = interpolated_tape.to_json();
        let expected_json = r#"{"test1":1.5,"test2":0,"test3":-1.5,"test4":0,"test5":1.5}"#;
        assert_eq!(json_output, expected_json);

        Ok(())
    }

    #[test]
    fn test_comparison_operators() -> Result<(), Box<dyn std::error::Error>> {
        let data = br#"
@width = 768
@height = 512
test_gt > @[ width ]
test_lt < @[ height ]
test_gte >= @[ 700 ]
test_lte <= @[ 500 ]
test_ne != @[ 999 ]
test_exact == @[ 42 ]
test_exists ?= @[ 100 ]
test_eq = @[ width ]
"#;

        let tape = TextTape::from_slice(data)?;

        let interpolated_tape =
            InterpolatedTape::from_tape_with_interpolation(&tape, Encoding::Utf8)?;
        let json_output = interpolated_tape.to_json();

        let expected_json = r#"{"test_gt":{"GREATER_THAN":768},"test_lt":{"LESS_THAN":512},"test_gte":{"GREATER_THAN_EQUAL":700},"test_lte":{"LESS_THAN_EQUAL":500},"test_ne":{"NOT_EQUAL":999},"test_exact":42,"test_exists":100,"test_eq":768}"#;
        assert_eq!(json_output, expected_json);

        Ok(())
    }

    #[test]
    fn test_direct_variable_reference() -> Result<(), Box<dyn std::error::Error>> {
        let data = br#"
@var_a = 0.7
@var_b = @var_a
test_obj = {
    edge_color_mult = @var_b
}
"#;

        let tape = TextTape::from_slice(data)?;
        let interpolated_tape =
            InterpolatedTape::from_tape_with_interpolation(&tape, Encoding::Utf8)?;

        let json_output = interpolated_tape.to_json();
        let expected_json = r#"{"test_obj":{"edge_color_mult":0.7}}"#;
        assert_eq!(json_output, expected_json);

        Ok(())
    }

    #[test]
    fn test_forward_variable_reference() -> Result<(), Box<dyn std::error::Error>> {
        let data = br#"
@derived_value = @base_value
@base_value = 42
result = @derived_value
"#;

        let tape = TextTape::from_slice(data)?;
        let interpolated_tape =
            InterpolatedTape::from_tape_with_interpolation(&tape, Encoding::Utf8)?;

        let json_output = interpolated_tape.to_json();
        let expected_json = r#"{"result":42}"#;
        assert_eq!(json_output, expected_json);

        Ok(())
    }

    #[test]
    fn test_unresolved_variable_reference_error() -> Result<(), Box<dyn std::error::Error>> {
        // Test that unresolved references produce clear error messages
        let data = br#"
@missing_ref = @nonexistent_var
test = @missing_ref
"#;

        let tape = TextTape::from_slice(data)?;
        let result = InterpolatedTape::from_tape_with_interpolation(&tape, Encoding::Utf8);

        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("Unresolved variable references"));
        assert!(error_msg.contains("@missing_ref -> @nonexistent_var"));

        Ok(())
    }

    #[test]
    fn test_interpolation_with_duplicate_keys_preserve() -> Result<(), Box<dyn std::error::Error>> {
        let data = br#"
@var = 10
duplicate = @var
duplicate = @[var * 2]
test = @[var + 5]
"#;

        let tape = TextTape::from_slice(data)?;
        let interpolated_tape =
            InterpolatedTape::from_tape_with_interpolation(&tape, Encoding::Utf8)?;

        // Test with preserve mode (default)
        let options = jomini::json::JsonOptions::new()
            .with_duplicate_keys(jomini::json::DuplicateKeyMode::Preserve);

        let mut output = Vec::new();
        interpolated_tape.to_writer_with_options(&mut output, options, Encoding::Utf8)?;
        let json_output = String::from_utf8(output)?;

        // Should preserve duplicate keys and filter out @var
        let expected = r#"{"duplicate":10,"duplicate":20,"test":15}"#;
        assert_eq!(json_output, expected);

        Ok(())
    }

    #[test]
    fn test_interpolation_with_duplicate_keys_group() -> Result<(), Box<dyn std::error::Error>> {
        let data = br#"
@var = 10
duplicate = @var
duplicate = @[var * 2]
test = @[var + 5]
"#;

        let tape = TextTape::from_slice(data)?;
        let interpolated_tape =
            InterpolatedTape::from_tape_with_interpolation(&tape, Encoding::Utf8)?;

        // Test with group mode
        let options = jomini::json::JsonOptions::new()
            .with_duplicate_keys(jomini::json::DuplicateKeyMode::Group);

        let mut output = Vec::new();
        interpolated_tape.to_writer_with_options(&mut output, options, Encoding::Utf8)?;
        let json_output = String::from_utf8(output)?;

        // Should group duplicate keys and filter out @var
        let expected = r#"{"duplicate":[10,20],"test":15}"#;
        assert_eq!(json_output, expected);

        Ok(())
    }

    #[test]
    fn test_interpolation_with_duplicate_keys_key_value_pairs(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let data = br#"
@var = 10
duplicate = @var
duplicate = @[var * 2]
test = @[var + 5]
"#;

        let tape = TextTape::from_slice(data)?;
        let interpolated_tape =
            InterpolatedTape::from_tape_with_interpolation(&tape, Encoding::Utf8)?;

        // Test with key-value-pairs mode
        let options = jomini::json::JsonOptions::new()
            .with_duplicate_keys(jomini::json::DuplicateKeyMode::KeyValuePairs);

        let mut output = Vec::new();
        interpolated_tape.to_writer_with_options(&mut output, options, Encoding::Utf8)?;
        let json_output = String::from_utf8(output)?;

        // Should use key-value-pairs format and filter out @var
        let expected = r#"{"type":"obj","val":[["duplicate",10],["duplicate",20],["test",15]]}"#;
        assert_eq!(json_output, expected);

        Ok(())
    }

    #[test]
    fn test_interpolation_with_operators_and_grouping() -> Result<(), Box<dyn std::error::Error>> {
        let data = br#"
@threshold = 50
condition > @threshold
condition < @[threshold * 2]
condition = @[threshold / 2]
"#;

        let tape = TextTape::from_slice(data)?;
        let interpolated_tape =
            InterpolatedTape::from_tape_with_interpolation(&tape, Encoding::Utf8)?;

        // Test with group mode and operators
        let options = jomini::json::JsonOptions::new()
            .with_duplicate_keys(jomini::json::DuplicateKeyMode::Group);

        let mut output = Vec::new();
        interpolated_tape.to_writer_with_options(&mut output, options, Encoding::Utf8)?;
        let json_output = String::from_utf8(output)?;

        // Should group conditions and handle operators, filter out @threshold
        let expected = r#"{"condition":[{"GREATER_THAN":50},{"LESS_THAN":100},25]}"#;
        assert_eq!(json_output, expected);

        Ok(())
    }
}
