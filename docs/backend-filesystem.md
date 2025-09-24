## FilesystemBackend

The `FilesystemBackend` class provides a way to store and retrieve files from the local filesystem. It is part of a
larger system that manages file storage, allowing users to save files to a specified directory and access them later.

### High-Level Description

The `FilesystemBackend` class is responsible for handling file operations on the local filesystem. It allows users to
specify a base directory where files will be stored. The class provides methods to save files, retrieve files, and
manage the storage directory.

### Key Features

- **Base Directory Management**: Users can specify a base directory for file storage. If the directory does not exist,
  it will be created automatically.
- **File Operations**: The class provides methods to save files to the base directory and retrieve them later.
- **Error Handling**: The class includes error handling to manage issues such as permission errors or invalid paths.
