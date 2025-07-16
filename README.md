# Artifacts Store

```mermaid
graph LR
    A[prompt] --> B[generator]
    B --> C[serialize]
    C --> D[deserialize]
```

### prompt

> multiple times per artifact

`prompt.<name>` ends up as `$prompt/<name>` in the generator script.

### file

> multiple times per artifact

`file.<name>.path` will be the handle of the file on the target system.

### serialize

> once per artifact

Defaults to `artifacts.config.serialize.default`, but can be overwritten.

### deserialize

> once per artifact

Defaults to `artifacts.config.deserialize.default`, but can be overwritten.

### shared

> once per artifact

handles what the `$out` variable will be usually it will be
`machines/<machine>/` from `nixosConfigurations.<machine>`, but with this
enabled it will be `shared/`

## Queue

- Check if you can deserialize all artifacts `=> $out/<artifact>/<files>`
  - if all artifacts exist the end up in `$out/<artifact>/<files>`
  - if not all artifacts are possible to deserialize, run the generator
    - generate `$out/<artifact>/<files>`
    - serialize generated secrets
      `$out/<artifacts>/<files> => $serialized/<artifacts>/<files>`
- copy
  `$out/<artifacts>/<files> => target-system/<artifact-store>/<artifact>/<files>`

> You can **skip** serialization/deserialization, which ends up generating a new
> artifact every time
