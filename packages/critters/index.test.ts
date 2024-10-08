import { Critters } from "./dist/index.js";
import { test, expect } from "vitest";

test("basic functionality", () => {
	const critters = new Critters();

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

	expect(inlined).toContain(".blue");
	expect(inlined).not.toContain(".red");
});
