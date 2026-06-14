# Snapshot file
# Unset all aliases to avoid conflicts with functions
unalias -a 2>/dev/null || true
shopt -s expand_aliases
# Check for rg availability
if ! (unalias rg 2>/dev/null; command -v rg) >/dev/null 2>&1; then
  function rg {
  local _cc_bin="${CLAUDE_CODE_EXECPATH:-}"
  [[ -x $_cc_bin ]] || _cc_bin=/c/Users/kusta/.local/bin/claude.exe
  if [[ ! -x $_cc_bin ]]; then command rg "$@"; return; fi
  if [[ -n $ZSH_VERSION ]]; then
    ARGV0=rg "$_cc_bin" "$@"
  elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "win32" ]]; then
    ARGV0=rg "$_cc_bin" "$@"
  elif [[ $BASHPID != $$ ]]; then
    exec -a rg "$_cc_bin" "$@"
  else
    (exec -a rg "$_cc_bin" "$@")
  fi
}
fi
export PATH='/c/Users/kusta/bin:/mingw64/bin:/usr/local/bin:/usr/bin:/bin:/mingw64/bin:/usr/bin:/c/Users/kusta/bin:/c/Program Files/Eclipse Adoptium/jdk-21.0.10.7-hotspot/bin:/c/Program Files/Common Files/Oracle/Java/javapath:/c/windows/system32:/c/windows:/c/windows/System32/Wbem:/c/windows/System32/WindowsPowerShell/v1.0:/c/windows/System32/OpenSSH:/cmd:/c/Program Files/nodejs:/c/Program Files/Docker/Docker/resources/bin:/c/Users/kusta/.cargo/bin:/c/Users/kusta/AppData/Local/Programs/Python/Launcher:/c/Users/kusta/.local/bin:/c/Users/kusta/AppData/Local/Microsoft/WindowsApps:/c/Users/kusta/AppData/Local/Programs/Microsoft VS Code/bin:/c/Users/kusta/AppData/Roaming/npm:/c/Users/kusta/AppData/Local/Python/bin:/c/Users/kusta/AppData/Local/Programs/Antigravity/bin:/c/Users/kusta/AppData/Local/Microsoft/WinGet/Packages/Google.Protobuf_Microsoft.Winget.Source_8wekyb3d8bbwe/bin:/usr/bin/vendor_perl:/usr/bin/core_perl:/c/Users/kusta/.claude/plugins/cache/claude-plugins-official/rust-analyzer-lsp/1.0.0/bin'
