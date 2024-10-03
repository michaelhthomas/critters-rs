# `@critters-rs/critters`

Node.js bindings for [`critters-rs`](https://github/com/michaelhthomas/critters-rs), a tool to quickly inline your site's critical CSS. Enables integration of `critters-rs` with the Javascript ecosystem through integrations. Aims to be an almost drop-in replacement for the original [`critters`](https://github.com/GoogleChromeLabs/critters).

## Usage

1. Install package
    ```
    pnpm add @critters-rs/critters
    ```
1. Process a file with `critters`
    ```ts
    import { Critters } from '@critters-rs/critters';

    const critters = new Critters({
      // configuration
    });
    
    const html = `
    <html>
    <head>
      <style>
        .red { color: red }
        .blue { color: blue }
      </style>
    </head>
    <body>
      <div class="blue">I'm Blue</div>
    </body>
    </html>
    `;

    const inlined = critters.process(html);
    
    console.log(inlined);
    // <html>
    // <head>
    //   <style>.blue{color:blue;}</style>
    // </head>
    // <body>
    //   <div class=\"blue\">I'm Blue</div>
    // </body>
    // </html>
    ```
