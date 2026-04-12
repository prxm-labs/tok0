#compdef tok0

autoload -U is-at-least

_tok0() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
":: :_tok0_commands" \
"*::: :->tok0" \
&& ret=0
    case $state in
    (tok0)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:tok0-command-$line[1]:"
        case $line[1] in
            (stats)
_arguments "${_arguments_options[@]}" : \
'--format=[]:FORMAT:_default' \
'--graph[]' \
'--history[]' \
'--daily[]' \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(git)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
'*::command:_default' \
&& ret=0
;;
(read)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
'*::args:_default' \
&& ret=0
;;
(ls)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
'*::args:_default' \
&& ret=0
;;
(grep)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
'*::args:_default' \
&& ret=0
;;
(find)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
'*::args:_default' \
&& ret=0
;;
(diff)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
'*::args:_default' \
&& ret=0
;;
(smart)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
'*::args:_default' \
&& ret=0
;;
(proxy)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
'*::args:_default' \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(doctor)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(rule)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_tok0__rule_commands" \
"*::: :->rule" \
&& ret=0

    case $state in
    (rule)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:tok0-rule-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(show)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
':name -- Rule name (matches the `name` field in the \[filter\] section):_default' \
&& ret=0
;;
(test)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
':name -- Rule name to exercise:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_tok0__rule__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:tok0-rule-help-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(show)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(test)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(init)
_arguments "${_arguments_options[@]}" : \
'-g[]' \
'--global[]' \
'--uninstall[]' \
'--show[]' \
'--wizard[Run the guided onboarding wizard]' \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(update)
_arguments "${_arguments_options[@]}" : \
'--check[]' \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(discover)
_arguments "${_arguments_options[@]}" : \
'--since=[]:SINCE:_default' \
'--all[]' \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(rewrite)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
'*::args:_default' \
&& ret=0
;;
(profile)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
'*::args:_default' \
&& ret=0
;;
(ext)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_tok0__ext_commands" \
"*::: :->ext" \
&& ret=0

    case $state in
    (ext)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:tok0-ext-command-$line[1]:"
        case $line[1] in
            (install)
_arguments "${_arguments_options[@]}" : \
'--name=[]:NAME:_default' \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
':url:_default' \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_tok0__ext__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:tok0-ext-help-command-$line[1]:"
        case $line[1] in
            (install)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(telemetry)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_tok0__telemetry_commands" \
"*::: :->telemetry" \
&& ret=0

    case $state in
    (telemetry)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:tok0-telemetry-command-$line[1]:"
        case $line[1] in
            (on)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(off)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_tok0__telemetry__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:tok0-telemetry-help-command-$line[1]:"
        case $line[1] in
            (on)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(off)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(auth)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_tok0__auth_commands" \
"*::: :->auth" \
&& ret=0

    case $state in
    (auth)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:tok0-auth-command-$line[1]:"
        case $line[1] in
            (login)
_arguments "${_arguments_options[@]}" : \
'--token=[API token (or set TOK0_API_KEY env var)]:TOKEN:_default' \
'--api-url=[Cloud API URL]:API_URL:_default' \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(logout)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_tok0__auth__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:tok0-auth-help-command-$line[1]:"
        case $line[1] in
            (login)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(logout)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(cloud)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_tok0__cloud_commands" \
"*::: :->cloud" \
&& ret=0

    case $state in
    (cloud)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:tok0-cloud-command-$line[1]:"
        case $line[1] in
            (team)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_tok0__cloud__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:tok0-cloud-help-command-$line[1]:"
        case $line[1] in
            (team)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(completions)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
':shell -- Shell to generate completions for:(bash elvish fish powershell zsh)' \
&& ret=0
;;
(trust)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(untrust)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(verify)
_arguments "${_arguments_options[@]}" : \
'*-v[Increase verbosity (-v, -vv, -vvv)]' \
'*--verbose[Increase verbosity (-v, -vv, -vvv)]' \
'-u[Ultra-compact output mode]' \
'--ultra-compact[Ultra-compact output mode]' \
'-h[Print help]' \
'--help[Print help]' \
':path -- Path to the hook file:_default' \
':hash -- Expected SHA-256 hash:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_tok0__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:tok0-help-command-$line[1]:"
        case $line[1] in
            (stats)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(git)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(read)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(ls)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(grep)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(find)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(diff)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(smart)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(proxy)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(doctor)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(rule)
_arguments "${_arguments_options[@]}" : \
":: :_tok0__help__rule_commands" \
"*::: :->rule" \
&& ret=0

    case $state in
    (rule)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:tok0-help-rule-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(show)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(test)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(init)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(update)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(discover)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(rewrite)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(profile)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(ext)
_arguments "${_arguments_options[@]}" : \
":: :_tok0__help__ext_commands" \
"*::: :->ext" \
&& ret=0

    case $state in
    (ext)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:tok0-help-ext-command-$line[1]:"
        case $line[1] in
            (install)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(telemetry)
_arguments "${_arguments_options[@]}" : \
":: :_tok0__help__telemetry_commands" \
"*::: :->telemetry" \
&& ret=0

    case $state in
    (telemetry)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:tok0-help-telemetry-command-$line[1]:"
        case $line[1] in
            (on)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(off)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(auth)
_arguments "${_arguments_options[@]}" : \
":: :_tok0__help__auth_commands" \
"*::: :->auth" \
&& ret=0

    case $state in
    (auth)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:tok0-help-auth-command-$line[1]:"
        case $line[1] in
            (login)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(logout)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(cloud)
_arguments "${_arguments_options[@]}" : \
":: :_tok0__help__cloud_commands" \
"*::: :->cloud" \
&& ret=0

    case $state in
    (cloud)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:tok0-help-cloud-command-$line[1]:"
        case $line[1] in
            (team)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(completions)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(trust)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(untrust)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(verify)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
}

(( $+functions[_tok0_commands] )) ||
_tok0_commands() {
    local commands; commands=(
'stats:Token savings analytics' \
'git:Git operations' \
'read:Smart file reading' \
'ls:Directory listing' \
'grep:Search files' \
'find:Find files' \
'diff:File diff' \
'smart:Smart code summary' \
'proxy:Proxy passthrough (no filtering, tracking only)' \
'status:Show tok0 status\: version, hooks, savings, trust, extensions' \
'doctor:Diagnose configuration + hook-health problems' \
'rule:Inspect and test compression rules' \
'init:Initialize hooks for AI tools' \
'update:Check for updates' \
'discover:Discover missed savings opportunities' \
'rewrite:Rewrite command (used by hooks)' \
'profile:Profile compression pipeline timing' \
'ext:Manage extension rule packs' \
'telemetry:Control anonymous telemetry (on/off/status)' \
'auth:Authenticate with tok0 cloud for team analytics' \
'cloud:View team dashboard (requires auth)' \
'completions:Generate shell completions' \
'trust:Trust a project directory for local filter rules' \
'untrust:Remove trust from a project directory' \
'verify:Verify hook file integrity' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'tok0 commands' commands "$@"
}
(( $+functions[_tok0__auth_commands] )) ||
_tok0__auth_commands() {
    local commands; commands=(
'login:Login with API token' \
'status:Show authentication status' \
'logout:Remove stored credentials and disable cloud reporting' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'tok0 auth commands' commands "$@"
}
(( $+functions[_tok0__auth__help_commands] )) ||
_tok0__auth__help_commands() {
    local commands; commands=(
'login:Login with API token' \
'status:Show authentication status' \
'logout:Remove stored credentials and disable cloud reporting' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'tok0 auth help commands' commands "$@"
}
(( $+functions[_tok0__auth__help__help_commands] )) ||
_tok0__auth__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 auth help help commands' commands "$@"
}
(( $+functions[_tok0__auth__help__login_commands] )) ||
_tok0__auth__help__login_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 auth help login commands' commands "$@"
}
(( $+functions[_tok0__auth__help__logout_commands] )) ||
_tok0__auth__help__logout_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 auth help logout commands' commands "$@"
}
(( $+functions[_tok0__auth__help__status_commands] )) ||
_tok0__auth__help__status_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 auth help status commands' commands "$@"
}
(( $+functions[_tok0__auth__login_commands] )) ||
_tok0__auth__login_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 auth login commands' commands "$@"
}
(( $+functions[_tok0__auth__logout_commands] )) ||
_tok0__auth__logout_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 auth logout commands' commands "$@"
}
(( $+functions[_tok0__auth__status_commands] )) ||
_tok0__auth__status_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 auth status commands' commands "$@"
}
(( $+functions[_tok0__cloud_commands] )) ||
_tok0__cloud_commands() {
    local commands; commands=(
'team:Show team savings summary' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'tok0 cloud commands' commands "$@"
}
(( $+functions[_tok0__cloud__help_commands] )) ||
_tok0__cloud__help_commands() {
    local commands; commands=(
'team:Show team savings summary' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'tok0 cloud help commands' commands "$@"
}
(( $+functions[_tok0__cloud__help__help_commands] )) ||
_tok0__cloud__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 cloud help help commands' commands "$@"
}
(( $+functions[_tok0__cloud__help__team_commands] )) ||
_tok0__cloud__help__team_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 cloud help team commands' commands "$@"
}
(( $+functions[_tok0__cloud__team_commands] )) ||
_tok0__cloud__team_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 cloud team commands' commands "$@"
}
(( $+functions[_tok0__completions_commands] )) ||
_tok0__completions_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 completions commands' commands "$@"
}
(( $+functions[_tok0__diff_commands] )) ||
_tok0__diff_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 diff commands' commands "$@"
}
(( $+functions[_tok0__discover_commands] )) ||
_tok0__discover_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 discover commands' commands "$@"
}
(( $+functions[_tok0__doctor_commands] )) ||
_tok0__doctor_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 doctor commands' commands "$@"
}
(( $+functions[_tok0__ext_commands] )) ||
_tok0__ext_commands() {
    local commands; commands=(
'install:Install extension from a git URL' \
'list:List installed extensions' \
'remove:Remove an installed extension' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'tok0 ext commands' commands "$@"
}
(( $+functions[_tok0__ext__help_commands] )) ||
_tok0__ext__help_commands() {
    local commands; commands=(
'install:Install extension from a git URL' \
'list:List installed extensions' \
'remove:Remove an installed extension' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'tok0 ext help commands' commands "$@"
}
(( $+functions[_tok0__ext__help__help_commands] )) ||
_tok0__ext__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 ext help help commands' commands "$@"
}
(( $+functions[_tok0__ext__help__install_commands] )) ||
_tok0__ext__help__install_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 ext help install commands' commands "$@"
}
(( $+functions[_tok0__ext__help__list_commands] )) ||
_tok0__ext__help__list_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 ext help list commands' commands "$@"
}
(( $+functions[_tok0__ext__help__remove_commands] )) ||
_tok0__ext__help__remove_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 ext help remove commands' commands "$@"
}
(( $+functions[_tok0__ext__install_commands] )) ||
_tok0__ext__install_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 ext install commands' commands "$@"
}
(( $+functions[_tok0__ext__list_commands] )) ||
_tok0__ext__list_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 ext list commands' commands "$@"
}
(( $+functions[_tok0__ext__remove_commands] )) ||
_tok0__ext__remove_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 ext remove commands' commands "$@"
}
(( $+functions[_tok0__find_commands] )) ||
_tok0__find_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 find commands' commands "$@"
}
(( $+functions[_tok0__git_commands] )) ||
_tok0__git_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 git commands' commands "$@"
}
(( $+functions[_tok0__grep_commands] )) ||
_tok0__grep_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 grep commands' commands "$@"
}
(( $+functions[_tok0__help_commands] )) ||
_tok0__help_commands() {
    local commands; commands=(
'stats:Token savings analytics' \
'git:Git operations' \
'read:Smart file reading' \
'ls:Directory listing' \
'grep:Search files' \
'find:Find files' \
'diff:File diff' \
'smart:Smart code summary' \
'proxy:Proxy passthrough (no filtering, tracking only)' \
'status:Show tok0 status\: version, hooks, savings, trust, extensions' \
'doctor:Diagnose configuration + hook-health problems' \
'rule:Inspect and test compression rules' \
'init:Initialize hooks for AI tools' \
'update:Check for updates' \
'discover:Discover missed savings opportunities' \
'rewrite:Rewrite command (used by hooks)' \
'profile:Profile compression pipeline timing' \
'ext:Manage extension rule packs' \
'telemetry:Control anonymous telemetry (on/off/status)' \
'auth:Authenticate with tok0 cloud for team analytics' \
'cloud:View team dashboard (requires auth)' \
'completions:Generate shell completions' \
'trust:Trust a project directory for local filter rules' \
'untrust:Remove trust from a project directory' \
'verify:Verify hook file integrity' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'tok0 help commands' commands "$@"
}
(( $+functions[_tok0__help__auth_commands] )) ||
_tok0__help__auth_commands() {
    local commands; commands=(
'login:Login with API token' \
'status:Show authentication status' \
'logout:Remove stored credentials and disable cloud reporting' \
    )
    _describe -t commands 'tok0 help auth commands' commands "$@"
}
(( $+functions[_tok0__help__auth__login_commands] )) ||
_tok0__help__auth__login_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help auth login commands' commands "$@"
}
(( $+functions[_tok0__help__auth__logout_commands] )) ||
_tok0__help__auth__logout_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help auth logout commands' commands "$@"
}
(( $+functions[_tok0__help__auth__status_commands] )) ||
_tok0__help__auth__status_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help auth status commands' commands "$@"
}
(( $+functions[_tok0__help__cloud_commands] )) ||
_tok0__help__cloud_commands() {
    local commands; commands=(
'team:Show team savings summary' \
    )
    _describe -t commands 'tok0 help cloud commands' commands "$@"
}
(( $+functions[_tok0__help__cloud__team_commands] )) ||
_tok0__help__cloud__team_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help cloud team commands' commands "$@"
}
(( $+functions[_tok0__help__completions_commands] )) ||
_tok0__help__completions_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help completions commands' commands "$@"
}
(( $+functions[_tok0__help__diff_commands] )) ||
_tok0__help__diff_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help diff commands' commands "$@"
}
(( $+functions[_tok0__help__discover_commands] )) ||
_tok0__help__discover_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help discover commands' commands "$@"
}
(( $+functions[_tok0__help__doctor_commands] )) ||
_tok0__help__doctor_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help doctor commands' commands "$@"
}
(( $+functions[_tok0__help__ext_commands] )) ||
_tok0__help__ext_commands() {
    local commands; commands=(
'install:Install extension from a git URL' \
'list:List installed extensions' \
'remove:Remove an installed extension' \
    )
    _describe -t commands 'tok0 help ext commands' commands "$@"
}
(( $+functions[_tok0__help__ext__install_commands] )) ||
_tok0__help__ext__install_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help ext install commands' commands "$@"
}
(( $+functions[_tok0__help__ext__list_commands] )) ||
_tok0__help__ext__list_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help ext list commands' commands "$@"
}
(( $+functions[_tok0__help__ext__remove_commands] )) ||
_tok0__help__ext__remove_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help ext remove commands' commands "$@"
}
(( $+functions[_tok0__help__find_commands] )) ||
_tok0__help__find_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help find commands' commands "$@"
}
(( $+functions[_tok0__help__git_commands] )) ||
_tok0__help__git_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help git commands' commands "$@"
}
(( $+functions[_tok0__help__grep_commands] )) ||
_tok0__help__grep_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help grep commands' commands "$@"
}
(( $+functions[_tok0__help__help_commands] )) ||
_tok0__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help help commands' commands "$@"
}
(( $+functions[_tok0__help__init_commands] )) ||
_tok0__help__init_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help init commands' commands "$@"
}
(( $+functions[_tok0__help__ls_commands] )) ||
_tok0__help__ls_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help ls commands' commands "$@"
}
(( $+functions[_tok0__help__profile_commands] )) ||
_tok0__help__profile_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help profile commands' commands "$@"
}
(( $+functions[_tok0__help__proxy_commands] )) ||
_tok0__help__proxy_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help proxy commands' commands "$@"
}
(( $+functions[_tok0__help__read_commands] )) ||
_tok0__help__read_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help read commands' commands "$@"
}
(( $+functions[_tok0__help__rewrite_commands] )) ||
_tok0__help__rewrite_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help rewrite commands' commands "$@"
}
(( $+functions[_tok0__help__rule_commands] )) ||
_tok0__help__rule_commands() {
    local commands; commands=(
'list:List all active rules (builtin + extensions + project-local)' \
'show:Show full config for a single rule by name' \
'test:Apply a rule to stdin and print the before/after savings' \
    )
    _describe -t commands 'tok0 help rule commands' commands "$@"
}
(( $+functions[_tok0__help__rule__list_commands] )) ||
_tok0__help__rule__list_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help rule list commands' commands "$@"
}
(( $+functions[_tok0__help__rule__show_commands] )) ||
_tok0__help__rule__show_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help rule show commands' commands "$@"
}
(( $+functions[_tok0__help__rule__test_commands] )) ||
_tok0__help__rule__test_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help rule test commands' commands "$@"
}
(( $+functions[_tok0__help__smart_commands] )) ||
_tok0__help__smart_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help smart commands' commands "$@"
}
(( $+functions[_tok0__help__stats_commands] )) ||
_tok0__help__stats_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help stats commands' commands "$@"
}
(( $+functions[_tok0__help__status_commands] )) ||
_tok0__help__status_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help status commands' commands "$@"
}
(( $+functions[_tok0__help__telemetry_commands] )) ||
_tok0__help__telemetry_commands() {
    local commands; commands=(
'on:Enable anonymous telemetry' \
'off:Disable anonymous telemetry' \
'status:Show current telemetry status' \
    )
    _describe -t commands 'tok0 help telemetry commands' commands "$@"
}
(( $+functions[_tok0__help__telemetry__off_commands] )) ||
_tok0__help__telemetry__off_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help telemetry off commands' commands "$@"
}
(( $+functions[_tok0__help__telemetry__on_commands] )) ||
_tok0__help__telemetry__on_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help telemetry on commands' commands "$@"
}
(( $+functions[_tok0__help__telemetry__status_commands] )) ||
_tok0__help__telemetry__status_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help telemetry status commands' commands "$@"
}
(( $+functions[_tok0__help__trust_commands] )) ||
_tok0__help__trust_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help trust commands' commands "$@"
}
(( $+functions[_tok0__help__untrust_commands] )) ||
_tok0__help__untrust_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help untrust commands' commands "$@"
}
(( $+functions[_tok0__help__update_commands] )) ||
_tok0__help__update_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help update commands' commands "$@"
}
(( $+functions[_tok0__help__verify_commands] )) ||
_tok0__help__verify_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 help verify commands' commands "$@"
}
(( $+functions[_tok0__init_commands] )) ||
_tok0__init_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 init commands' commands "$@"
}
(( $+functions[_tok0__ls_commands] )) ||
_tok0__ls_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 ls commands' commands "$@"
}
(( $+functions[_tok0__profile_commands] )) ||
_tok0__profile_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 profile commands' commands "$@"
}
(( $+functions[_tok0__proxy_commands] )) ||
_tok0__proxy_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 proxy commands' commands "$@"
}
(( $+functions[_tok0__read_commands] )) ||
_tok0__read_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 read commands' commands "$@"
}
(( $+functions[_tok0__rewrite_commands] )) ||
_tok0__rewrite_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 rewrite commands' commands "$@"
}
(( $+functions[_tok0__rule_commands] )) ||
_tok0__rule_commands() {
    local commands; commands=(
'list:List all active rules (builtin + extensions + project-local)' \
'show:Show full config for a single rule by name' \
'test:Apply a rule to stdin and print the before/after savings' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'tok0 rule commands' commands "$@"
}
(( $+functions[_tok0__rule__help_commands] )) ||
_tok0__rule__help_commands() {
    local commands; commands=(
'list:List all active rules (builtin + extensions + project-local)' \
'show:Show full config for a single rule by name' \
'test:Apply a rule to stdin and print the before/after savings' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'tok0 rule help commands' commands "$@"
}
(( $+functions[_tok0__rule__help__help_commands] )) ||
_tok0__rule__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 rule help help commands' commands "$@"
}
(( $+functions[_tok0__rule__help__list_commands] )) ||
_tok0__rule__help__list_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 rule help list commands' commands "$@"
}
(( $+functions[_tok0__rule__help__show_commands] )) ||
_tok0__rule__help__show_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 rule help show commands' commands "$@"
}
(( $+functions[_tok0__rule__help__test_commands] )) ||
_tok0__rule__help__test_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 rule help test commands' commands "$@"
}
(( $+functions[_tok0__rule__list_commands] )) ||
_tok0__rule__list_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 rule list commands' commands "$@"
}
(( $+functions[_tok0__rule__show_commands] )) ||
_tok0__rule__show_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 rule show commands' commands "$@"
}
(( $+functions[_tok0__rule__test_commands] )) ||
_tok0__rule__test_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 rule test commands' commands "$@"
}
(( $+functions[_tok0__smart_commands] )) ||
_tok0__smart_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 smart commands' commands "$@"
}
(( $+functions[_tok0__stats_commands] )) ||
_tok0__stats_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 stats commands' commands "$@"
}
(( $+functions[_tok0__status_commands] )) ||
_tok0__status_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 status commands' commands "$@"
}
(( $+functions[_tok0__telemetry_commands] )) ||
_tok0__telemetry_commands() {
    local commands; commands=(
'on:Enable anonymous telemetry' \
'off:Disable anonymous telemetry' \
'status:Show current telemetry status' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'tok0 telemetry commands' commands "$@"
}
(( $+functions[_tok0__telemetry__help_commands] )) ||
_tok0__telemetry__help_commands() {
    local commands; commands=(
'on:Enable anonymous telemetry' \
'off:Disable anonymous telemetry' \
'status:Show current telemetry status' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'tok0 telemetry help commands' commands "$@"
}
(( $+functions[_tok0__telemetry__help__help_commands] )) ||
_tok0__telemetry__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 telemetry help help commands' commands "$@"
}
(( $+functions[_tok0__telemetry__help__off_commands] )) ||
_tok0__telemetry__help__off_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 telemetry help off commands' commands "$@"
}
(( $+functions[_tok0__telemetry__help__on_commands] )) ||
_tok0__telemetry__help__on_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 telemetry help on commands' commands "$@"
}
(( $+functions[_tok0__telemetry__help__status_commands] )) ||
_tok0__telemetry__help__status_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 telemetry help status commands' commands "$@"
}
(( $+functions[_tok0__telemetry__off_commands] )) ||
_tok0__telemetry__off_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 telemetry off commands' commands "$@"
}
(( $+functions[_tok0__telemetry__on_commands] )) ||
_tok0__telemetry__on_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 telemetry on commands' commands "$@"
}
(( $+functions[_tok0__telemetry__status_commands] )) ||
_tok0__telemetry__status_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 telemetry status commands' commands "$@"
}
(( $+functions[_tok0__trust_commands] )) ||
_tok0__trust_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 trust commands' commands "$@"
}
(( $+functions[_tok0__untrust_commands] )) ||
_tok0__untrust_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 untrust commands' commands "$@"
}
(( $+functions[_tok0__update_commands] )) ||
_tok0__update_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 update commands' commands "$@"
}
(( $+functions[_tok0__verify_commands] )) ||
_tok0__verify_commands() {
    local commands; commands=()
    _describe -t commands 'tok0 verify commands' commands "$@"
}

if [ "$funcstack[1]" = "_tok0" ]; then
    _tok0 "$@"
else
    compdef _tok0 tok0
fi
