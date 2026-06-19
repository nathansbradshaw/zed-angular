# Zed Angular Extension

## Overview

**Note: This project is currently a work in progress. Expect potential bugs or issues.**

This extension integrates the Angular Language Service into Zed. It uses the same options that Angular applies during compilation. To ensure the most accurate information, enable the `strictTemplates` option in the `tsconfig.json` of the angular project as shown in below:

```json
"angularCompilerOptions": {
  "strictTemplates": true
}
```

## Version Management

The extension depends on the `@angular/language-server` and `typescript` Node packages. It tries to **automatically match the Angular version of your project**, so it works out of the box from older Angular (v13) up to the latest release — no per-project configuration needed.

The version is resolved in the following order of priority:

1. **Manual override** — versions you set in `initialization_options` (see below) always win.
2. **Project's own language server** — if `@angular/language-server` and `typescript` are already installed in your project's `node_modules` (including monorepos with hoisted dependencies), the extension uses them directly, guaranteeing an exact match with your project.
3. **Derived from `package.json`** — otherwise, the extension reads the `@angular/core` (or `@angular/language-service`) version from your project's `package.json`, derives a compatible `@angular/language-server` + `typescript` pair, and installs it.
4. **Defaults** — if nothing can be detected, it falls back to a modern default.

You can still pin versions explicitly (useful for monorepos or unusual setups where auto-detection doesn't apply):

```json
{
  "lsp": {
    "angular": {
      "initialization_options": {
        "angular_language_server_version": "17.3.0",
        "typescript_version": "5.3.2"
      }
    }
  }
}
```

Please ensure the versions of Angular and TypeScript are compatible to avoid issues.

Refer to [Angular Version Compatibility](https://angular.dev/reference/versions#unsupported-angular-versions) for details. Mismatched versions may lead to bugs, so it is not recommended to use `latest` as a version, but it's also a valid option.

> Tip: to see which versions the extension resolved, open the Angular language server logs in Zed (command palette → `dev: open language server logs`). The extension logs lines prefixed with `[angular]`.

## Feature Settings

The same `initialization_options` block accepts the Angular Language Service settings, using the **exact dot-namespaced keys** of the official VSCode extension — so you can copy them straight from the Angular docs. All are optional.

```json
{
  "lsp": {
    "angular": {
      "initialization_options": {
        "angular.forceStrictTemplates": true,
        "angular.suggest.autoImports": true,
        "angular.suggest.includeCompletionsWithSnippetText": true,
        "angular.suggest.includeAutomaticOptionalChainCompletions": true,
        "angular.suppressAngularDiagnosticCodes": "-992008,-998001",
        "angular.server.useClientSideFileWatcher": false,

        "angular.inlayHints.variableTypes.ifAliasTypes": true,
        "angular.inlayHints.bindingHints.pipeOutputTypes": true,
        "angular.inlayHints.eventHints.parameterTypes": true,
        "angular.inlayHints.controlFlowHints.switchExpressionTypes": true
      }
    }
  }
}
```

Notes:

- **Auto-imports** (`angular.suggest.autoImports`) and a useful set of **inlay hints** are enabled by default; set the corresponding key to `false` to turn one off.
- **`--angularCoreVersion`** is detected automatically from your project's `package.json` and forwarded to the compiler, keeping diagnostics accurate on older Angular versions — you don't need to set anything.
- The full list of `angular.inlayHints.*` keys matches the [VSCode extension](https://marketplace.visualstudio.com/items?itemName=Angular.ng-template) (groups: `variableTypes`, `bindingHints`, `eventHints`, `functionTypes`, `parameterHints`, `controlFlowHints`, `interaction`). Any key you set is forwarded verbatim to the language server.

### Not available (yet)

These rely on editor-specific command APIs that Zed does not expose to extensions, so they can't be provided here even though the VSCode extension has them:

- **Go to component / Go to template** commands.
- **View Template Typecheck Block** (`getTemplateTcb`).
- **Restart Angular server** command (Zed manages language server restarts itself).

Everything else — completions, diagnostics, hover, go-to-definition, document symbols, rename, find references, and inlay hints — comes from the shared `@angular/language-server` and works in Zed.

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
