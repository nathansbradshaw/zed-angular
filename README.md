# Zed Angular Extension

## Overview

This extension integrates the Angular Language Service into Zed. It uses the same options that Angular applies during compilation. To ensure the most accurate information, enable the `strictTemplates` option in the `tsconfig.json` of the angular project as shown in below:

```json
"angularCompilerOptions": {
  "strictTemplates": true
}
```

## Requirements

**This extension does not bundle or download a language server. **

**Why?** the version of the angular language server is sensitive to your project's angular version. A misalignment between them can create all kinds of unexpected problems. If this extension were to maintain its own copy, it's almost guaranteeing that it will be a problem (this is especially the case for people who are maintaining multiple angular applications).

Therefore, this extension will look for and run the copy of `@angular/language-server` installed in your project. Install it alongside `typescript` as dev dependencies:

```sh
npm install --save-dev @angular/language-server typescript
```

By default the extension looks for the package at `node_modules/@angular/language-server`, relative to the root of the worktree you have open in Zed.

## Version Management

The extension depends on the `@angular/language-server` and `typescript` Node packages. It will use whatever versions of each that are available locally in your project.

The major version of `@angular/language-server` must match the Angular major version used by your project. TypeScript must be **5.0 or later**, and **6.0.3 is the latest supported version** — newer releases are untested and may fail to load. Mismatches typically surface as a `Failed to resolve 'typescript/lib/tsserverlibrary'` error in the language server logs.

If your project would otherwise pull in a newer TypeScript, pin it:

```json
{
  "devDependencies": {
    "typescript": "~6.0.3"
  }
}
```

Refer to [Angular Version Compatibility](https://angular.dev/reference/versions#unsupported-angular-versions) for details.

## Configuration

All options are set under `lsp.angular.initialization_options` in your Zed `settings.json` (or a project-local `.zed/settings.json`):

| Option                         | Type     | Default                                  | Description                                                                        |
| ------------------------------ | -------- | ---------------------------------------- | ---------------------------------------------------------------------------------- |
| `angular_language_server_path` | `string` | `node_modules/@angular/language-server`  | Location of the `@angular/language-server` package directory.                      |
| `max_ts_server_memory`         | `number` | unset (node default, ~4 GB)              | Heap limit in MB, passed to node as `--max-old-space-size`.                         |

Both can be combined — this is the typical monorepo setup, where the app lives in a subfolder *and* the project is large enough to exhaust node's default heap:

```json
{
  "lsp": {
    "angular": {
      "initialization_options": {
        "angular_language_server_path": "client/node_modules/@angular/language-server",
        "max_ts_server_memory": 8192
      }
    }
  }
}
```

Both options are optional and independent; omit either one to keep its default.

### Custom Server Path

Set `angular_language_server_path` when the language server is not installed at the default location — for example in a monorepo where the Angular app lives in a subfolder.

The value must be the **package directory**, not the `index.js` file inside it (a trailing `/index.js` is tolerated and stripped). Accepted forms:

| Form              | Example                                                        | Resolved against                    |
| ----------------- | -------------------------------------------------------------- | ----------------------------------- |
| Worktree-relative | `client/node_modules/@angular/language-server`                 | The root of the open worktree       |
| Absolute          | `/Users/me/repo/client/node_modules/@angular/language-server`  | Used as-is                          |
| Home-relative     | `~/.npm-global/lib/node_modules/@angular/language-server`      | `$HOME` from your shell environment |

The path is **not validated** by the extension. Zed extensions run sandboxed and can only inspect files present in the project's file index, which excludes gitignored trees such as `node_modules`, so any existence check would report false negatives. If the path is wrong, Node reports a `MODULE_NOT_FOUND` error in the language server logs instead.

TypeScript and Angular are then probed in the worktree root, its `node_modules`, and the ancestors of the resolved package directory — so a server under `client/` still resolves `client/node_modules/typescript` correctly.

### Memory

In large workspaces (e.g. monorepos), the language server can exceed node's default heap limit (~4 GB) and crash repeatedly. If the server keeps restarting or stops responding after a few minutes, raise the limit with `max_ts_server_memory` (in MB, passed to node as `--max-old-space-size`):

```json
{
  "lsp": {
    "angular": {
      "initialization_options": {
        "max_ts_server_memory": 8192
      }
    }
  }
}
```

Start at `8192` and increase only if crashes persist; the value is a ceiling, not a reservation, so node allocates lazily. Setting it above the machine's available RAM will trade crashes for swapping. The flag is emitted before the server script path so node interprets it, and it is omitted entirely when the option is unset.

## Installation Instructions

To install this extension locally:

1. Clone this repository.
2. Open the Zed editor and navigate to the Extensions window.
3. Click on "Install Dev Extension."
4. Select the cloned repository location and complete the installation.
5. Add a language server list definition to the HTML and TypeScript language settings. In `settings.json`, add the following _(ellipsis is a valid value in settings, use it as shown)_:

```json
{
  "languages": {
    "TypeScript": {
      "language_servers": ["angular", "..."]
    },
    "HTML": {
      "language_servers": ["angular", "..."]
    }
  }
}
```

If the published version of the extension is already installed, Zed uninstalls it before installing the dev extension. After changing the source, run `zed: rebuild dev extension` from the command palette — the extension is compiled to WebAssembly at install/rebuild time, so edits are not picked up until you do.
