---
name: unity-csharp-standards
description: Unity C# naming and code style. Use when writing, editing, reviewing or refactoring C# scripts in the Unity project or its packages (.cs files under unity/), and UI Toolkit USS files.
---

# Unity C# naming and code style

Based on Unity's *Naming and code style tips for C# scripting in Unity*
(https://unity.com/how-to/naming-and-code-style-tips-c-scripting-unity),
which follows Microsoft's C# conventions.

## The `.editorconfig`

The `.editorconfig` beside this file puts these rules into the editor:
Rider, Visual Studio and `dotnet format` read it and flag code that
breaks them.

- Copy it, unchanged, into the Unity project's root folder: the one
  holding `Assets/`, `Packages/` and `ProjectSettings/`. An editor
  applies the nearest `.editorconfig` found walking up from each file,
  so every script under `Assets/` follows it. Its `root = true` stops
  the search there.
- Packages the project references through `file:` entries in
  `Packages/manifest.json` sit outside that folder and are not covered.
  When the project has such packages, put the file in a folder above
  both instead. In this repo that is `unity/projects/`, which holds the
  project `cart-pole/` and its `packages/`.
- In Rider, the naming rules apply only once *Settings → Editor →
  Inspection Settings → Read settings from editorconfig, project
  settings and rule sets* is on. The formatting rules apply without it.

## Identifiers

- No special characters in identifiers: no backslashes, symbols or
  Unicode. Some Unity command-line tools break on them.
- No Hungarian notation (`iCounter`, `strPlayerName`).
- Casing:
  - `PascalCase` for types, methods, public fields, properties, events,
    enum names and values, and namespaces.
  - `camelCase` for local variables and parameters, with no prefix.
  - `kebab-case` for UI Toolkit USS names.

## Fields and variables

- Name variables with nouns. They are things or states.
- Booleans start with a verb that makes them a question: `isDead`,
  `isWalking`, `hasDamageMultiplier`.
- Use meaningful names and don't abbreviate. Single letters are fine
  only for loop counters and maths. Put the important word first where
  it reads better (`HorizontalAlignment`, not `AlignmentHorizontal`),
  and favour readability over brevity (`CanScrollHorizontally`, not
  `ScrollableX`).
- No redundant names: in a class `Player`, a member is `Score`, not
  `PlayerScore`.
- No jokes or puns in names.
- Prefixes:
  - private member fields: `m_` + camelCase (`m_movementSpeed`);
  - static fields: `s_` + camelCase;
  - constants: `k_` + PascalCase (`k_MaxHealth`);
  - public fields and properties: no prefix.
- Prefer properties with a public getter to public fields.
- Always write the access modifier, `private` included.
- One variable declaration per line.
- Drop redundant initialisers: no `= 0` on numbers, `= null` on
  references or `= false` on booleans. Fields start at those values.
- Use `var` only for local variables where it helps readability and the
  type is obvious from the right-hand side. Never where it hides the
  type, and not for primitive types outside a loop.

## Enums

- `PascalCase` for the enum name and its values, with no prefix or
  suffix.
- The name is a singular noun (`WeaponType`), because a value is one of
  the set.
- A `[Flags]` enum is the exception: its name is plural
  (`AttackModes`), because a value can hold several.
- A public enum may sit outside any class to be used everywhere.

## Classes and interfaces

- Class names are `PascalCase` nouns or noun phrases.
- A file holding a `MonoBehaviour` is named after it, and holds only
  that one `MonoBehaviour`.
- Interface names are `I` followed by an adjective for what they do:
  `IKillable`, `IDamageable<T>`.

## Methods

- Names start with a verb or verb phrase, with context where needed:
  `GetDirection`, `FindTarget`.
- Parameters are `camelCase`, like local variables.
- A method returning `bool` asks a question: `IsGameOver`,
  `HasStartedTurn`.

## Events and event handlers

- Name an event with a verb phrase. Present participle for "before",
  past participle for "after": `OpeningDoor`, `DoorOpened`.
- Use `System.Action` / `Action<T>` for events.
- The method that raises an event starts with `On`: `OnDoorOpened()`
  invokes `DoorOpened?.Invoke()`.
- An observer's handling method is the subject's name, an underscore,
  then the event: `GameEvents_DoorOpened`.
- Create a custom `EventArgs` type only when the event must carry data.
  Derive it from `System.EventArgs`, or make it a read-only struct.

## Namespaces

- `PascalCase`, with no symbols or underscores.
- Use sub-namespaces separated by dots (`MyApplication.GameFlow`,
  `MyApplication.AI`), mirroring the project's folder structure.
- Add a `using` directive at the top of the file rather than repeating
  the namespace. Write the namespace prefix only where two types with
  the same name meet.
