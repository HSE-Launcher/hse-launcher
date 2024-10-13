# Modpack Builder Specification File Format

The `spec.json` file is used to define the specifications for generating modpacks. Below is the format of the `spec.json` file and an explanation of each field.

## JSON Structure

```json
{
  "download_server_base": "string",
  "resources_url_base": "string",
  "versions": [
    {
      "name": "string",
      "minecraft_version": "string",
      "loader_name": "string",
      "loader_version": "string",
      "include": ["string"],
      "include_no_overwrite": ["string"],
      "include_from": "string",
      "replace_download_urls": "boolean",
      "auth_provider": {
        "type": "string",
        "data_field1": "data_value1",
        "other_data_fields": "other_data_values"
      },
      "exec_before": "string",
      "exec_after": "string"
    }
  ],
  "exec_before_all": "string",
  "exec_after_all": "string"
}
```

## Fields

### Root Fields

- **download_server_base**: The base URL where the modpack will be deployed. All files in the generated folder (`generated` by default) must be accessible by `<download_server_base>/<file_relative_path>` after deployment. For example, the version manifest has to be at `<download_server_base>/modpacks/version_manifest.json`.
- **resources_url_base**: The base URL for assets (optional). Should be equal to `<download_server_base>/assets/objects` if the generated folder structure is not changed after upload.
- **versions**: An array of version specifications (see below for details).
- **exec_before_all**: A bash command to execute before processing all versions (optional).
- **exec_after_all**: A bash command to execute after processing all versions (optional). Can be used to automatically deploy the generated files in any possible way (e.g., by `rsync`'ing them to the server with `nginx`).

### Version Fields

- **name**: The name of the version.
- **minecraft_version**: The Minecraft version for this modpack.
- **loader_name**: The name of the mod loader (e.g., "vanilla", "fabric", "forge").
- **loader_version**: The version of the mod loader (optional; latest for fabric and `recommended` for forge if not set).
- **include**: A list of additional files or directories to include in the modpack (optional; e.g., mods).
- **include_no_overwrite**: A list of files or directories to include without overwriting existing files (optional; e.g., configs).
- **include_from**: A directory from which to include files (optional).
- **replace_download_urls**: A boolean indicating whether to replace download URLs (e.g., of vanilla libraries or assets).
- **auth_provider**: Authentication data for accessing protected resources (optional).
  - **type**: The authentication provider name (e.g., "telegram" for [this telegram format](https://foxlab.dev/minecraft/tgauth-backend)).
  - Any additional fields for the selected authentication provider.
- **exec_before**: A command to execute before processing this version (optional).
- **exec_after**: A command to execute after processing this version (optional).

[Example](spec.json.example)
