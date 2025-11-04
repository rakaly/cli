pub fn eu4_tokens_resolver() -> eu4save::BasicTokenResolver {
    let data = include_bytes!("../assets/tokens/eu4.txt");
    eu4save::BasicTokenResolver::from_text_lines(&data[..]).expect("embedded tokens invalid format")
}

pub fn ck3_tokens_resolver() -> ck3save::BasicTokenResolver {
    let data = include_bytes!("../assets/tokens/ck3.txt");
    ck3save::BasicTokenResolver::from_text_lines(&data[..]).expect("embedded tokens invalid format")
}

pub fn vic3_tokens_resolver() -> vic3save::BasicTokenResolver {
    let data = include_bytes!("../assets/tokens/vic3.txt");
    vic3save::BasicTokenResolver::from_text_lines(&data[..])
        .expect("embedded tokens invalid format")
}

pub fn imperator_tokens_resolver() -> imperator_save::BasicTokenResolver {
    let data = include_bytes!("../assets/tokens/imperator.txt");
    imperator_save::BasicTokenResolver::from_text_lines(&data[..])
        .expect("embedded tokens invalid format")
}

pub fn hoi4_tokens_resolver() -> hoi4save::BasicTokenResolver {
    let data = include_bytes!("../assets/tokens/hoi4.txt");
    hoi4save::BasicTokenResolver::from_text_lines(&data[..])
        .expect("embedded tokens invalid format")
}

pub fn eu5_tokens_resolver() -> eu5save::BasicTokenResolver {
    let data = include_bytes!("../assets/tokens/eu5.txt");
    eu5save::BasicTokenResolver::from_text_lines(&data[..])
        .expect("embedded tokens invalid format")
}
