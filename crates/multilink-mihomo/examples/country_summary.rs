//! Show only country counts; never print subscription addresses or node names.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(data) = multilink_platform::load()? else {
        println!("Нет сохранённых серверов");
        return Ok(());
    };
    let settings: multilink_core::Settings = serde_json::from_slice(&data)?;
    for (code, name) in multilink_core::available_countries(&settings.nodes) {
        let count = settings
            .nodes
            .iter()
            .filter(|n| {
                n.enabled && multilink_core::node_country(&n.name).as_deref() == Some(code.as_str())
            })
            .count();
        println!("{code} {name}: {count}");
    }
    println!(
        "Без определённой страны: {}",
        settings
            .nodes
            .iter()
            .filter(|n| n.enabled && multilink_core::node_country(&n.name).is_none())
            .count()
    );
    let mut flags = std::collections::BTreeMap::new();
    for node in settings
        .nodes
        .iter()
        .filter(|n| n.enabled && multilink_core::node_country(&n.name).is_none())
    {
        let chars: Vec<_> = node.name.chars().collect();
        for pair in chars.windows(2) {
            if pair.iter().all(|c| ('\u{1f1e6}'..='\u{1f1ff}').contains(c)) {
                let flag: String = pair.iter().collect();
                *flags.entry(flag).or_insert(0usize) += 1;
            }
        }
    }
    for (flag, count) in flags {
        println!("Неопределённый флаг {flag}: {count}");
    }
    Ok(())
}
