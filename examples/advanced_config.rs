use critters_rs::{Critters, CrittersOptions, PreloadStrategy};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html_content = r#"
        <html>
            <head>
                <link rel="stylesheet" href="styles.css" />
            </head>
            <body>
                <div class="critical">Hello World</div>
            </body>
        </html>
    "#;

    let options = CrittersOptions {
        path: "./dist".to_string(),
        external: true,
        preload: PreloadStrategy::Swap,
        inline_fonts: true,
        preload_fonts: true,
        compress: true,
        ..Default::default()
    };

    let critters = Critters::new(options);
    let result = critters.process(html_content)?;
    
    println!("Processed HTML:\n{}", result);

    Ok(())
}