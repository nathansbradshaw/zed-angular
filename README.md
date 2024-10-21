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

## Versioning

The language service extension relies on the `@angular/language-service` and `typescript` packages

for its backend. `@angular/language-service` is always bundled with the extension, and is always
the latest version at the time of the release. This is managed by this extension.
For more information, please see [#594](https://github.com/angular/vscode-ng-language-service/issues/594).

## How to use

- Clone this repo locally
- Open `Zed` extensions window
- Click `Install Dev Extension`
- Point to the location of the cloned repo.
