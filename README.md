# Zed Angular Extension

## Overview

**Note: This project is currently a work in progress. Expect potential bugs or issues.**

This extension integrates the Angular Language Service into Zed. It uses the same options that Angular applies during compilation. To ensure the most accurate information, enable the `strictTemplates` option in the `tsconfig.json` of the angular project  as shown in below:

```json
"angularCompilerOptions": {
  "strictTemplates": true
}
```

## Version Management

The extension depends on `@angular/language-service` and `typescript` Node packages. By default, these versions are managed internally, but they can be customized using the following settings:

```json
{
  "lsp": {
    "angular": {
      "initialization_options": {
        "angular_language_service_version": "17.3.0",
        "typescript_version": "5.3.2"
      }
    }
  }
}
```

Please ensure the versions of Angular and TypeScript are compatible to avoid issues.

Refer to [Angular Version Compatibility](https://angular.dev/reference/versions#unsupported-angular-versions) for details. Mismatched versions may lead to bugs, so it is not recommended to use `latest` as a version, but it's also a valid option.  

## Installation Instructions

To install this extension locally:

1. Clone this repository.
2. Open the Zed editor and navigate to the Extensions window.
3. Click on "Install Dev Extension."
4. Select the cloned repository location and complete the installation.
