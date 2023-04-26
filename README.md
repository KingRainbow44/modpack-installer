# Modpack Installer
A simple modpack installer for Minecraft.

# Usage
To use, you can either:
- Download a `modpack.json` to the same directory as the installer
- Rename the installer to a direct-download URL to a `modpack.json` file
  - To rename the installer, use the format:
    - `-` = `/`
    - `;` = `:`

## Modpack Manifest
```json
{
  "name": "gamin'",
  "version": "1.0.0",

  "target": "1.19.4",
  "fabric": "0.14.18",

  "loader": "fabric-loader-0.14.18-1.19.4",
  "folder": "1.19.4-gamin",

  "mods": ["P7dR8mSH", "AANobbMI"],
  "external": [
    {
      "url": "https://crepe.moe/c/1",
      "file": "config/crepe.moe"
    }
  ]
}
```

### Fields
- `mods` is an array with the project IDs from [Modrinth](https://modrinth.com)
- `external` is an array of objects with the following fields:
  - `url` is the URL to download the file from
  - `file` is the path to save the file to
  - **This field can be used for malicious purposes, never install an unknown modpack!**
- `target` is the Minecraft version to install the modpack for
- `fabric` is the Fabric version to install the modpack for
- `loader` is the name of the Fabric Loader installation folder
- `folder` is the name of the modpack installation folder