# zed-angular

The Angular Language Service uses the same set of options that are used to compile the application.
To get the most complete information in the editor, set the `strictTemplates` option in `tsconfig.json`,
as shown in the following example:

```
"angularCompilerOptions": {
  "strictTemplates": true
}
```

# THIS IS STILL A WIP

Dev notes
The original attempt at an angular lsp can be found here https://github.com/nathansbradshaw/zed/blob/a857bc14c9fabf4fe374715fba88e98cd800c0c2/crates/zed/src/languages/angular.rs

The angular LSP is very similar to the Vue lsp https://github.com/zed-industries/zed/blob/main/extensions/vue/src/vue.rs

TODO:

- ~Find the TypeScript install~
- Correctly identify angular projects
- correct the SCM files
- remove this todo list
