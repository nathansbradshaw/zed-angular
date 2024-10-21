# zed-angular

## THIS IS STILL A WORK IN PROGRESS EXPECT BUGS!

The Angular Language Service uses the same set of options that are used to compile the application.
To get the most complete information in the editor, set the `strictTemplates` option in `tsconfig.json`,
as shown in the following example:

```json
"angularCompilerOptions": {
  "strictTemplates": true
}
```

## Versioning (TODO - Currently it's based on your workspace)

The language service extension relies on the `@angular/language-service` and `typescript` packages

for its backend. `@angular/language-service` is always bundled with the extension, and is always
the latest version at the time of the release.
`typescript` is loaded, in order of priority, from:

1. The path specified by `typescript.tsdk` in project or global settings.
2. _(Recommended)_ The version of `typescript` bundled with the Angular Language Service extension.
3. The version of `typescript` present in the current workspace's node_modules.

We suggest **not** specifying `typescript.tsdk` in your VSCode settings
per method (1) above. If the `typescript` package is loaded by methods (1) or (3), there is a potential
for a mismatch between the API expected by `@angular/language-service` and the API provided by `typescript`.
This could lead to a failure of the language service extension.

For more information, please see [#594](https://github.com/angular/vscode-ng-language-service/issues/594).

## How to use

- Clone this repo locally
- Open `Zed` extensions window
- Click `Install Dev Extension`
- Point to the location of the cloned repo.
