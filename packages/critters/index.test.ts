import { expect, test } from "vitest";
import { Critters } from "./dist/index.js";

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

test("options parsing", () => {
	const critters = new Critters({
		compress: false,
		mergeStylesheets: false,
		pruneSource: true,
		preload: "None",
		keyframes: "All",
	});

	const html = `
  <html>
  <head>
    <style>
      .red { color: red; }
      .blue { color: blue; }
      @keyframes fadeIn {
        from { opacity: 0; }
        to { opacity: 1; }
      }
    </style>
  </head>
  <body>
    <div class="blue">I'm Blue</div>
  </body>
  </html>
  `;

	const inlined = critters.process(html);

	// Should contain the critical .blue class
	expect(inlined).toContain(".blue");

	// Should not contain unused .red class
	expect(inlined).not.toContain(".red");

	// Should include keyframes since keyframes: "all" is set
	expect(inlined).toContain("@keyframes fadeIn");
	expect(inlined).toContain("opacity");

	// Should not be compressed (compress: false)
	// Uncompressed CSS should have spaces and formatting
	expect(inlined).toMatch(/\.blue\s+{\n/);
});
