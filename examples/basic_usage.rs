use critters_rs::{Critters, CrittersOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new Critters instance with default options
    let critters = Critters::new(CrittersOptions::default());

    // Process HTML content to extract critical CSS
    let html = r#"
        <html>
            <head>
                <style>
                    .critical { color: red; }
                    .unused { color: blue; }
                </style>
            </head>
            <body>
                <div class="critical">Hello World</div>
            </body>
        </html>
    "#;

    let processed_html = critters.process(html)?;
    // The processed HTML will only contain the .critical style, with .unused removed
    println!("Processed HTML:\n{}", processed_html);

    Ok(())
}