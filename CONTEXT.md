# FANTOM Librarian

The FANTOM Librarian context covers a musician's portable sound-material library and the app
installations used to work with it.

## Language

**Workspace**:
The portable folder that is the user's library, including its catalog, managed originals, and
exports. It is separate from an application installation.
_Avoid_: library database, app data

**Personal installation**:
The release installation a musician uses for their real workspace and daily library work.
_Avoid_: production app, real app

**Development installation**:
The separately identified build used while developing and testing the librarian. It has its own
application state and only opens a personal workspace after explicit user confirmation.
_Avoid_: debug app, test app
