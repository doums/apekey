[![apekey](https://img.shields.io/github/actions/workflow/status/doums/apekey/deploy.yml?color=0D0D0D&logoColor=BFBFBF&labelColor=404040&logo=github&style=for-the-badge)](https://github.com/doums/apekey/actions?query=workflow%3Aapekey)
[![apekey](https://img.shields.io/aur/version/apekey?color=0D0D0D&logoColor=BFBFBF&labelColor=404040&logo=arch-linux&style=for-the-badge)](https://aur.archlinux.org/packages/apekey/)

## apekey

List and browse your XMonad keymap.

<img src="https://user-images.githubusercontent.com/6359431/211675677-0e8b44d4-7551-4da4-9d5a-51c83c95b895.png" width="650">

### Install

- latest [release](https://github.com/doums/apekey/releases/latest)
- AUR [package](https://aur.archlinux.org/packages/apekey)

### Usage

⚠ For now apekey only supports keybindings specified in
_emacs-style_ format
([EZConfig](https://xmonad.github.io/xmonad-docs/xmonad-contrib/XMonad-Util-EZConfig.html))

Apekey reads your `xmonad.hs` config and looks for comments with
special formats. Based on these comments, apekey will parse and
generate the keymap, and will render it in a dedicated window.

##### `-- # [Title]`

Tell apekey to start parsing from here. An optional title can be
given. Use a second comment `-- #` to mark the end of the
keybindings declaration area.

```haskell
-- # XMonad keymap

-- your keybindings declaration

-- somewhere below
-- #
```

##### `-- ## Section`

Define a section of keybindings. All subsequent annotated keybinds
will belong to this section until another section is defined.

```haskell
-- ## Basics
-- a keybind declaration
-- a keybind declaration
-- a keybind declaration

-- ## Another section
-- keybindings declarations...
```

##### `-- Keybind description`

Adds a description to a keybinding. That is, a regular comment.
The next line must be the corresponding keybinding declaration.
Apekey will automatically parse and extract the keybinding from
it.

```haskell
-- Kill current window
, ("M-x",         kill)
```

##### `-- "<keys>" Description`

Some keybindings are not declared "inline" or using the emacs format.
e.g. mouse binding, workspaces/topics/screen navigation bindings
etc... are common cases. For these it is not possible to use the
simple `-- description` comments.

Instead, you can use this comment format to arbitrary write _fake_
keybindings.

```haskell
-- "M-<Topic key>" Move to topic x
-- "M-S-<Topic key>" Move current window to topic x
```

##### `-- ! Keybind ignored`

Annotate a keybind but do not render it.

```haskell
  -- ! Description
  , ("<M-u>",   spawn "script.sh")
```

#### Illustrative example

`xmonad.hs`

```haskell
-- # XMonad keymap
keybinds = ([
  -- ## Basics
  -- Recompile and restart XMonad
    ("M-C-q",       spawn "xmonad --recompile; xmonad --restart")
  -- Refresh XMonad
  , ("M-C-r",       refresh)
  -- Kill current window
  , ("M-x",         kill)

  -- ## Topic navigation
  -- "M-<Topic key>" Move to topic x
  -- "M-S-<Topic key>" Move current window to topic x
  -- Switch to last topic
  , ("M-<Tab>",     switchToLastTopic)

  -- ## Window navigation
  -- "M-↑→↓←" Navigate through windows
  -- "M-S-↑→↓←" Swap windows
  -- Focus next window up
  , ("M-k",         windows W.focusUp)
  -- Focus next window down
  , ("M-j",         windows W.focusDown)

  -- ...

-- #
```

Once you have added your descriptions simply launch apekey. Press
`Tab` to fuzzy search keybindings by key and/or description.

### Configuration

Apekey will look for a config file at
`$XDG_CONFIG_HOME/apekey/apekey.toml`.

The most important option is `xmonad_config`, it must be set to
the path pointing to your `xmonad.hs` configuration file.

```toml
xmonad_config = "~/.config/xmonad/xmonad.hs"
```

Other available options are defined in `src/user_config.rs`, check
for `UserConfig` structure
https://github.com/doums/apekey/blob/main/src/user_config.rs

### TODO

- highlight fuzzy matches

### License

Mozilla Public License 2.0
