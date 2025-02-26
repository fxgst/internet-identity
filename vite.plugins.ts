import { assertNonNullish } from "@dfinity/utils";
import { readFileSync } from "fs";
import { minify } from "html-minifier-terser";
import { extname } from "path";
import { Plugin } from "vite";
import viteCompression from "vite-plugin-compression";

/**
 * Read the II canister ID from dfx's local state
 */
const readCanisterId = (): string => {
  const canisterIdsJsonFile = "./.dfx/local/canister_ids.json";

  try {
    const {
      internet_identity: { local: canisterId },
    } = JSON.parse(readFileSync(canisterIdsJsonFile, "utf-8"));

    assertNonNullish(
      canisterId,
      `Could not get canister ID from ${canisterIdsJsonFile}`
    );

    console.log("Read canister ID:", canisterId);

    return canisterId;
  } catch (e) {
    throw Error(`Could not get canister ID from ${canisterIdsJsonFile}: ${e}`);
  }
};

/**
 * Inject the II canister ID as a <script /> tag in index.html for local development. Will process
 * at most 1 script tag.
 */
export const injectCanisterIdPlugin = (): {
  name: "html-transform";
  transformIndexHtml(html: string): string;
} => ({
  name: "html-transform",
  transformIndexHtml(html): string {
    const rgx = /<script type="module" src="(?<src>[^"]+)"><\/script>/;

    return html.replace(rgx, (_match, src) => {
      return `<script data-canister-id="${readCanisterId()}" type="module" src="${src}"></script>`;
    });
  },
});

/**
 * GZip generated resources e.g. index.js => index.js.gz
 */
export const compression = (): Plugin =>
  viteCompression({
    // II canister only supports one content type per resource. That is why we remove the original file.
    deleteOriginFile: true,
    filter: (file: string): boolean =>
      ![".html", ".css", ".webp", ".png", ".ico", ".svg"].includes(
        extname(file)
      ),
  });

/**
 * Minify HTML
 */
export const minifyHTML = (): {
  name: "html-transform";
  transformIndexHtml(html: string): Promise<string>;
} => ({
  name: "html-transform",
  async transformIndexHtml(html): Promise<string> {
    return minify(html, { collapseWhitespace: true });
  },
});
