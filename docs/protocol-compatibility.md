# Protocol compatibility

The library keeps protocol compatibility behavior explicit through
`CompatibilityMode` and capability discovery.

- **Strict** requires `rpc.discover` and rejects unadvertised methods.
- **Compatible** accepts supported historical wire forms, including legacy
  notification prefixes, and permits extension calls.
- **Permissive** does not preflight method calls against discovery data.

Discovery schemas are retained verbatim and parsed tolerantly so server-specific
extensions are not discarded. Gamerule models preserve boolean, integer, and
legacy string forms.
