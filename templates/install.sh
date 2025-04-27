#!/usr/bin/env bash

#{# template engine Tera #}

set -euo pipefail
RUN_DIRECTORY="$PWD"
_QUIET={{ quiet | escape_shell }}
_FORCE={{ force | escape_shell }}
_CANONICAL_BINARY_NAME={{ app | escape_shell }}

_E_GENERIC_ERROR=10
_TMPDIR="$(mktemp -d)"
cd "$_TMPDIR"
trap "[ -d \"$_TMPDIR\" ] &&  _log DEBUG \"Removing $_TMPDIR\" && rm -rf \"$_TMPDIR\"" EXIT

INSTALL_LOG_LEVEL={{ log_level | escape_shell }}
case "$INSTALL_LOG_LEVEL" in
    TRACE)
      INSTALL_LOG_LEVEL=0
      set -x
    ;;
    DEBUG)
      INSTALL_LOG_LEVEL=1
    ;;
    INFO)
      INSTALL_LOG_LEVEL=2
    ;;
    WARN)
      INSTALL_LOG_LEVEL=3
    ;;
    ERROR)
      INSTALL_LOG_LEVEL=4
    ;;
    FATAL)
      INSTALL_LOG_LEVEL=5
    ;;
    *)
      INSTALL_LOG_LEVEL=2
      _log ERROR "invalid log level: $INSTALL_LOG_LEVEL, using INFO"
esac
      


_log() {
   case "$1" in
      DEBUG)
        [ "$INSTALL_LOG_LEVEL" -le 1 ] || return
        echo "DEBUG: $2" >&2
        ;;
      INFO)
        [ "$INSTALL_LOG_LEVEL" -le 2 ] || return
        echo "INFO: $2" >&2
        ;;
      WARN)
        [ "$INSTALL_LOG_LEVEL" -le 3 ] || return
        echo "WARN: $2" >&2
        ;;
      ERROR)
        [ "$INSTALL_LOG_LEVEL" -le 4 ] || return
        echo "ERROR: $2" >&2
        ;;
      FATAL)
        [ "$INSTALL_LOG_LEVEL" -le 5 ] || return
        echo "FATAL: $2" >&2
        exit 100
        ;;
      *)
        return 1
        ;;
   esac
}

_ask_choices() {
  local choice choices opt add_none add_quit idx
  opt="$(getopt -o "" --long "none,quit" -n "${FUNCNAME[0]}" -- "$@")"
  [ "$?" -eq 0 ] || {
    _log FATAL "invalid options"
    exit 1
  }
  eval set -- "$opt"
  add_none=false
  add_quit=false
  while true; do
    case "$1" in
      --quit)
        add_quit=true
        shift
        ;;
      --none)
        add_none=true
        shift
        ;;
      --)
        shift
        break
        ;;
    esac
  done
  choices=("$@")
  {% raw %}
  if [ "${#choices[@]}" -eq 0 ]; then
    _log FATAL "no choices provided"
    exit 1
  fi
  {% endraw %}

  idx=1
  for c in "${choices[@]}"; do
    printf "\t%s)\t%s\n" "$idx" "$c" 1>&2
    idx=$((idx + 1))
  done

  if [ "$add_none" = true ]; then
    printf "\tn)\tnone\n" 1>&2
  fi
  if [ "$add_quit" = true ]; then
    printf "\tq)\tquit\n" 1>&2
  fi

  printf "Enter choice: " 1>&2
  read -r choice

  local final_choices=()
  for c in $choice; do
    case "$choice" in
      [0-9]*)
        c=$((c - 1))
    esac
    final_choices+=("$c")
  done
  echo "${final_choices[@]}"
}


_urlget() {
  if command -v curl &> /dev/null; then
    curl -fsSL "$1" 2> /dev/null
  elif command -v wget &> /dev/null; then
    wget -qO- "$1" 2> /dev/null
  else
    _log ERROR "neither curl nor wget found, unable to download files"
    return "$_E_GENERIC_ERROR"
  fi
}
{% if (assets | length  > 0) %}
_urls=( {% for asset in assets %}{{ asset.url | escape_shell }} {% endfor %})
_filenames=( {% for asset in assets %}{{ asset.name | escape_shell }} {% endfor %})
_filetypes=( {% for asset in assets %}{{ asset.filetype | escape_shell }} {% endfor %})
_printables=( {% for asset in assets %}{{ asset.name ~ " (" ~ asset.filetype ~ ")" | escape_shell }} {% endfor %})

printf "Please select one of the following:\n"
choice="$(_ask_choices --quit "${_printables[@]}")"

case "$choice" in
  q|n)
    exit 0
    ;;
  [0-9]*)
    if ! [ "$choice" -lt {% raw %}"${#_urls[@]}"{% endraw %} ]; then
      _log FATAL "invalid choice: $choice"
    fi
    ;;
  *)
    _log FATAL "invalid choice: $choice"
    ;;
esac

printf "Downloading from %s to %s\n" "${_urls[$choice]}" "$_TMPDIR"
_type="${_filetypes[$choice]}"
case "$_type" in
  "binary" | "Deb installer")
    filename="${_filenames[$choice]}"
    saved_file="$_TMPDIR/$filename"
    _urlget "${_urls[$choice]}" > "$saved_file"

    if [ "$_type" = "Deb installer" ]; then
      if command -v dpkg &> /dev/null; then
        printf "trying to install with dpkg, this may prompt for sudo\n"
        dpkg -i "$saved_file" || sudo dpkg -i "$saved_file"
      else
        _log FATAL "dpkg not found, unable to install package"
      fi
    elif [ "$_type" = "binary" ]; then
      chmod +x "$saved_file"

      if [ -z "$_CANONICAL_BINARY_NAME" ]; then
        read -r -p "enter alternate binary name (default: $filename): " binary_name
        binary_name="${binary_name:-$filename}"
      else
        binary_name="$_CANONICAL_BINARY_NAME"
      fi
      read -r -p "enter alternate binary directory (default: $RUN_DIRECTORY/bin): " binary_dir
      binary_dir="${binary_dir:-$RUN_DIRECTORY/bin}"
      mkdir -p "$binary_dir"
      cp "$saved_file" "$binary_dir/$binary_name"
    else
      _log FATAL "invalid filetype: $_type"
    fi
    ;;
  "tar.gz")
    filename="${_filenames[$choice]}"
    _urlget "${_urls[$choice]}" | tar xz
    executable_files=( $( find . -type f -executable -exec printf '{} ' \; ) )
    {% raw %}
    if [ "${#executable_files[@]}" -e 0 ]; then  {# raw block here to allow for the comment looking shell op #}
    {% endraw %}
      _log FATAL "no executable files found in archive"
    else
      choices="$(_ask_choices --quit ${executable_files[@]})"
    fi
    for choice in $choices; do
      case "$choice" in
        [0-9]*)
          cp ${executable_files[$choice]} "$RUN_DIRECTORY/bin"
          ;;
      esac
    done
    ;;
  *)
    _log FATAL "invalid filetype: ${_filetypes[$choice]}"
    ;;
esac
{% else %}
_log FATAL "no assets found"
{% endif %}