use jomini_next::{text::ObjectReader, Scalar, TextTape, TextToken, Utf8Encoding};
use std::collections::{HashMap, HashSet};

/// Memory-efficient interpolated tape that only allocates strings for interpolated values
pub struct InterpolatedTape<'a> {
    original_tape: &'a TextTape<'a>,
    interpolated_strings: Vec<String>,
    token_overrides: HashMap<usize, usize>, // token_index -> string_index
    variable_declarations: HashSet<String>, // variable names that were declared
}

impl<'a> InterpolatedTape<'a> {
    /// Create a new InterpolatedTape from an existing tape with interpolations applied
    pub fn from_tape_with_interpolation(
        tape: &'a TextTape<'a>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut variables: HashMap<String, f64> = HashMap::new();
        let mut interpolated_strings = Vec::new();
        let mut token_overrides = HashMap::new();
        let mut skip_interpolation: HashSet<usize> = HashSet::new();
        let mut variable_declarations: HashSet<String> = HashSet::new();

        // First pass: collect variable definitions
        let tokens = tape.tokens();
        let mut i = 0;

        while i < tokens.len() {
            match &tokens[i] {
                TextToken::Unquoted(scalar) => {
                    let text = std::str::from_utf8(scalar.as_bytes())?;

                    // Variable definition: @var_name
                    if text.starts_with('@') && !text.starts_with("@[") {
                        let var_name = &text[1..];

                        // Look for the value (no = operator in tokens, it's consumed by parser)
                        if i + 1 < tokens.len() {
                            if let TextToken::Unquoted(value_scalar) = &tokens[i + 1] {
                                let value_text = std::str::from_utf8(value_scalar.as_bytes())?;

                                // Handle @var = @[expression] format
                                if value_text.starts_with("@[") && value_text.ends_with("]") {
                                    let expr = &value_text[2..value_text.len() - 1];
                                    let computed_value = eval_expression(expr, &variables)?;
                                    variables.insert(var_name.to_string(), computed_value);
                                    skip_interpolation.insert(i);
                                    // Mark this as a variable declaration
                                    variable_declarations.insert(format!("@{}", var_name));
                                }
                                // Handle @var = number format
                                else if let Ok(value) = parse_f64(value_scalar.as_bytes()) {
                                    variables.insert(var_name.to_string(), value);
                                    skip_interpolation.insert(i);
                                    // Mark this as a variable declaration
                                    variable_declarations.insert(format!("@{}", var_name));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }

        // Second pass: find and store interpolations
        i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                TextToken::Unquoted(scalar) => {
                    let text = std::str::from_utf8(scalar.as_bytes())?;

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
                _ => {}
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

    /// Get the token at the specified index, using interpolated value if available

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

    /// Generate JSON output using jomini's JSON serialization
    pub fn to_json(&self) -> String {
        let materialized = self.materialize();
        let tokens = materialized.create_tokens();
        let reader = ObjectReader::from_tokens(&tokens, Utf8Encoding::new());

        // Recursively filter variable declarations starting from root object
        self.filter_object_json(&reader)
    }

    /// Recursively filter variable declarations from an object
    fn filter_object_json(&self, obj_reader: &ObjectReader<jomini_next::Utf8Encoding>) -> String {
        let mut json_parts = Vec::new();

        for (key, _op, field_value) in obj_reader.fields() {
            let key_str = key.read_str();

            // Skip variable declarations at this level
            if self.variable_declarations.contains(&*key_str) {
                continue;
            }

            // Process the value - check if it's an object, array, or primitive
            let value_json = self.filter_value_json(&field_value);

            // Safely escape the key for JSON
            let escaped_key = key_str.replace('\\', "\\\\").replace('"', "\\\"");
            let field_json = format!("\"{}\":{}", escaped_key, value_json);
            json_parts.push(field_json);
        }

        format!("{{{}}}", json_parts.join(","))
    }

    /// Process a field value, handling objects, arrays, and primitives
    fn filter_value_json(
        &self,
        value: &jomini_next::text::ValueReader<jomini_next::Utf8Encoding>,
    ) -> String {
        // Use jomini's JSON conversion to determine the actual type
        let json_str = value.json().to_string();

        // If it starts with '[', it's an array
        if json_str.starts_with('[') {
            if let Ok(array_reader) = value.read_array() {
                let mut json_parts = Vec::new();
                for item in array_reader.values() {
                    let item_json = self.filter_value_json(&item);
                    json_parts.push(item_json);
                }
                return format!("[{}]", json_parts.join(","));
            }
        }

        // If it starts with '{', it's an object
        if json_str.starts_with('{') {
            if let Ok(obj_reader) = value.read_object() {
                return self.filter_object_json(&obj_reader);
            }
        }

        // For primitive values, use jomini's JSON conversion
        json_str
    }

    /// Generate pretty-printed JSON output

    /// Write JSON output with filtering directly to a writer with options
    pub fn to_writer_with_options<W: std::io::Write>(
        &self,
        mut writer: W,
        _options: jomini_next::json::JsonOptions,
    ) -> std::io::Result<()> {
        // For now, use the filtered JSON string approach until we can properly implement
        // token-level filtering that maintains structural integrity
        // The variable filtering is the most important feature
        let filtered_json = self.to_json();

        // TODO: Implement proper pretty printing and duplicate key handling
        // This requires more sophisticated token manipulation to maintain parse tree structure
        writer.write_all(filtered_json.as_bytes())
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
    Operator(jomini_next::text::Operator),
    End(usize),
    MixedContainer,
}

impl MaterializedTape {
    /// Generate JSON output directly using jomini's JSON serialization
    pub fn to_json_direct(&self) -> String {
        use jomini_next::json::JsonOptions;
        let tokens = self.create_tokens();
        let reader = ObjectReader::from_tokens(&tokens, Utf8Encoding::new());
        reader
            .json()
            .with_options(JsonOptions::default())
            .to_string()
    }

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

fn eval_addition_subtraction(
    expr: &str,
    variables: &HashMap<String, f64>,
) -> Result<f64, Box<dyn std::error::Error>> {
    // Handle + and - operations (lowest precedence)
    // For left-associativity with recursive descent, find the RIGHTMOST operator
    let expr = expr.trim();

    // Handle negative numbers at the start (but only if there's no other minus after it)
    if expr.starts_with('-') && !expr[1..].starts_with('(') && !expr[1..].contains('-') {
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
    if operand.starts_with('-') {
        let var_name = &operand[1..];
        if let Some(&value) = variables.get(var_name) {
            return Ok(-value);
        }
    }

    Err(format!("Unknown operand: {}", operand).into())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let interpolated_tape = InterpolatedTape::from_tape_with_interpolation(&tape)?;

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
        let interpolated_tape = InterpolatedTape::from_tape_with_interpolation(&tape)?;

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
        let interpolated_tape = InterpolatedTape::from_tape_with_interpolation(&tape)?;

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
            InterpolatedTape::from_tape_with_interpolation(&interpolated_tape_raw)?;

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
        let interpolated_tape = InterpolatedTape::from_tape_with_interpolation(&tape)?;

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
        let interpolated_tape = InterpolatedTape::from_tape_with_interpolation(&tape)?;

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
        let interpolated_tape = InterpolatedTape::from_tape_with_interpolation(&tape)?;

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
        let interpolated_tape = InterpolatedTape::from_tape_with_interpolation(&tape)?;

        let json_output = interpolated_tape.to_json();
        let expected_json = r#"{"result1":0.16666666666666666,"result2":1,"result3":1}"#;
        assert_eq!(json_output, expected_json);

        Ok(())
    }
}
