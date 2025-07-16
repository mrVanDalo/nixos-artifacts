# Artifacts Store

```mermaid
graph LR
    A[prompt] --> B[generator]
    B --> C[serialize]
    C --> D[deserialize]
    D --> E[upload]
```

# prompt

> multiple times per artifact

`prompt.<name>` ends up as `$prompt/<name>` in the generator script.

# file

> multiple times per artifact

`file.<name>.path` will be the handle of the file on the target system.

# serialize

> once times per artifact

Defaults to `artifacts.serialize.default`, but can be overwritten.

# deserialize

> once times per artifact

Defaults to `artifacts.deserialize.default`, but can be overwritten.

# upload

> once times per artifact

Defaults to `artifacts.upload.default`, but can be overwritten.
