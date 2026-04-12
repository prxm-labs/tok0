
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'tok0' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'tok0'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'tok0' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Token savings analytics')
            [CompletionResult]::new('git', 'git', [CompletionResultType]::ParameterValue, 'Git operations')
            [CompletionResult]::new('read', 'read', [CompletionResultType]::ParameterValue, 'Smart file reading')
            [CompletionResult]::new('ls', 'ls', [CompletionResultType]::ParameterValue, 'Directory listing')
            [CompletionResult]::new('grep', 'grep', [CompletionResultType]::ParameterValue, 'Search files')
            [CompletionResult]::new('find', 'find', [CompletionResultType]::ParameterValue, 'Find files')
            [CompletionResult]::new('diff', 'diff', [CompletionResultType]::ParameterValue, 'File diff')
            [CompletionResult]::new('smart', 'smart', [CompletionResultType]::ParameterValue, 'Smart code summary')
            [CompletionResult]::new('proxy', 'proxy', [CompletionResultType]::ParameterValue, 'Proxy passthrough (no filtering, tracking only)')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show tok0 status: version, hooks, savings, trust, extensions')
            [CompletionResult]::new('doctor', 'doctor', [CompletionResultType]::ParameterValue, 'Diagnose configuration + hook-health problems')
            [CompletionResult]::new('rule', 'rule', [CompletionResultType]::ParameterValue, 'Inspect and test compression rules')
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize hooks for AI tools')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Check for updates')
            [CompletionResult]::new('discover', 'discover', [CompletionResultType]::ParameterValue, 'Discover missed savings opportunities')
            [CompletionResult]::new('rewrite', 'rewrite', [CompletionResultType]::ParameterValue, 'Rewrite command (used by hooks)')
            [CompletionResult]::new('profile', 'profile', [CompletionResultType]::ParameterValue, 'Profile compression pipeline timing')
            [CompletionResult]::new('ext', 'ext', [CompletionResultType]::ParameterValue, 'Manage extension rule packs')
            [CompletionResult]::new('telemetry', 'telemetry', [CompletionResultType]::ParameterValue, 'Control anonymous telemetry (on/off/status)')
            [CompletionResult]::new('auth', 'auth', [CompletionResultType]::ParameterValue, 'Authenticate with tok0 cloud for team analytics')
            [CompletionResult]::new('cloud', 'cloud', [CompletionResultType]::ParameterValue, 'View team dashboard (requires auth)')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completions')
            [CompletionResult]::new('trust', 'trust', [CompletionResultType]::ParameterValue, 'Trust a project directory for local filter rules')
            [CompletionResult]::new('untrust', 'untrust', [CompletionResultType]::ParameterValue, 'Remove trust from a project directory')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify hook file integrity')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'tok0;stats' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'format')
            [CompletionResult]::new('--graph', '--graph', [CompletionResultType]::ParameterName, 'graph')
            [CompletionResult]::new('--history', '--history', [CompletionResultType]::ParameterName, 'history')
            [CompletionResult]::new('--daily', '--daily', [CompletionResultType]::ParameterName, 'daily')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;git' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;read' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;ls' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;grep' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;find' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;diff' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;smart' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;proxy' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;status' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;doctor' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;rule' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all active rules (builtin + extensions + project-local)')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show full config for a single rule by name')
            [CompletionResult]::new('test', 'test', [CompletionResultType]::ParameterValue, 'Apply a rule to stdin and print the before/after savings')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'tok0;rule;list' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;rule;show' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;rule;test' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;rule;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all active rules (builtin + extensions + project-local)')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show full config for a single rule by name')
            [CompletionResult]::new('test', 'test', [CompletionResultType]::ParameterValue, 'Apply a rule to stdin and print the before/after savings')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'tok0;rule;help;list' {
            break
        }
        'tok0;rule;help;show' {
            break
        }
        'tok0;rule;help;test' {
            break
        }
        'tok0;rule;help;help' {
            break
        }
        'tok0;init' {
            [CompletionResult]::new('-g', '-g', [CompletionResultType]::ParameterName, 'g')
            [CompletionResult]::new('--global', '--global', [CompletionResultType]::ParameterName, 'global')
            [CompletionResult]::new('--uninstall', '--uninstall', [CompletionResultType]::ParameterName, 'uninstall')
            [CompletionResult]::new('--show', '--show', [CompletionResultType]::ParameterName, 'show')
            [CompletionResult]::new('--wizard', '--wizard', [CompletionResultType]::ParameterName, 'Run the guided onboarding wizard')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;update' {
            [CompletionResult]::new('--check', '--check', [CompletionResultType]::ParameterName, 'check')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;discover' {
            [CompletionResult]::new('--since', '--since', [CompletionResultType]::ParameterName, 'since')
            [CompletionResult]::new('--all', '--all', [CompletionResultType]::ParameterName, 'all')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;rewrite' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;profile' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;ext' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('install', 'install', [CompletionResultType]::ParameterValue, 'Install extension from a git URL')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List installed extensions')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove an installed extension')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'tok0;ext;install' {
            [CompletionResult]::new('--name', '--name', [CompletionResultType]::ParameterName, 'name')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;ext;list' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;ext;remove' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;ext;help' {
            [CompletionResult]::new('install', 'install', [CompletionResultType]::ParameterValue, 'Install extension from a git URL')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List installed extensions')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove an installed extension')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'tok0;ext;help;install' {
            break
        }
        'tok0;ext;help;list' {
            break
        }
        'tok0;ext;help;remove' {
            break
        }
        'tok0;ext;help;help' {
            break
        }
        'tok0;telemetry' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('on', 'on', [CompletionResultType]::ParameterValue, 'Enable anonymous telemetry')
            [CompletionResult]::new('off', 'off', [CompletionResultType]::ParameterValue, 'Disable anonymous telemetry')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show current telemetry status')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'tok0;telemetry;on' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;telemetry;off' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;telemetry;status' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;telemetry;help' {
            [CompletionResult]::new('on', 'on', [CompletionResultType]::ParameterValue, 'Enable anonymous telemetry')
            [CompletionResult]::new('off', 'off', [CompletionResultType]::ParameterValue, 'Disable anonymous telemetry')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show current telemetry status')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'tok0;telemetry;help;on' {
            break
        }
        'tok0;telemetry;help;off' {
            break
        }
        'tok0;telemetry;help;status' {
            break
        }
        'tok0;telemetry;help;help' {
            break
        }
        'tok0;auth' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('login', 'login', [CompletionResultType]::ParameterValue, 'Login with API token')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show authentication status')
            [CompletionResult]::new('logout', 'logout', [CompletionResultType]::ParameterValue, 'Remove stored credentials and disable cloud reporting')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'tok0;auth;login' {
            [CompletionResult]::new('--token', '--token', [CompletionResultType]::ParameterName, 'API token (or set TOK0_API_KEY env var)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'Cloud API URL')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;auth;status' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;auth;logout' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;auth;help' {
            [CompletionResult]::new('login', 'login', [CompletionResultType]::ParameterValue, 'Login with API token')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show authentication status')
            [CompletionResult]::new('logout', 'logout', [CompletionResultType]::ParameterValue, 'Remove stored credentials and disable cloud reporting')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'tok0;auth;help;login' {
            break
        }
        'tok0;auth;help;status' {
            break
        }
        'tok0;auth;help;logout' {
            break
        }
        'tok0;auth;help;help' {
            break
        }
        'tok0;cloud' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('team', 'team', [CompletionResultType]::ParameterValue, 'Show team savings summary')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'tok0;cloud;team' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;cloud;help' {
            [CompletionResult]::new('team', 'team', [CompletionResultType]::ParameterValue, 'Show team savings summary')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'tok0;cloud;help;team' {
            break
        }
        'tok0;cloud;help;help' {
            break
        }
        'tok0;completions' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;trust' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;untrust' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;verify' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase verbosity (-v, -vv, -vvv)')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('--ultra-compact', '--ultra-compact', [CompletionResultType]::ParameterName, 'Ultra-compact output mode')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'tok0;help' {
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Token savings analytics')
            [CompletionResult]::new('git', 'git', [CompletionResultType]::ParameterValue, 'Git operations')
            [CompletionResult]::new('read', 'read', [CompletionResultType]::ParameterValue, 'Smart file reading')
            [CompletionResult]::new('ls', 'ls', [CompletionResultType]::ParameterValue, 'Directory listing')
            [CompletionResult]::new('grep', 'grep', [CompletionResultType]::ParameterValue, 'Search files')
            [CompletionResult]::new('find', 'find', [CompletionResultType]::ParameterValue, 'Find files')
            [CompletionResult]::new('diff', 'diff', [CompletionResultType]::ParameterValue, 'File diff')
            [CompletionResult]::new('smart', 'smart', [CompletionResultType]::ParameterValue, 'Smart code summary')
            [CompletionResult]::new('proxy', 'proxy', [CompletionResultType]::ParameterValue, 'Proxy passthrough (no filtering, tracking only)')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show tok0 status: version, hooks, savings, trust, extensions')
            [CompletionResult]::new('doctor', 'doctor', [CompletionResultType]::ParameterValue, 'Diagnose configuration + hook-health problems')
            [CompletionResult]::new('rule', 'rule', [CompletionResultType]::ParameterValue, 'Inspect and test compression rules')
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize hooks for AI tools')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Check for updates')
            [CompletionResult]::new('discover', 'discover', [CompletionResultType]::ParameterValue, 'Discover missed savings opportunities')
            [CompletionResult]::new('rewrite', 'rewrite', [CompletionResultType]::ParameterValue, 'Rewrite command (used by hooks)')
            [CompletionResult]::new('profile', 'profile', [CompletionResultType]::ParameterValue, 'Profile compression pipeline timing')
            [CompletionResult]::new('ext', 'ext', [CompletionResultType]::ParameterValue, 'Manage extension rule packs')
            [CompletionResult]::new('telemetry', 'telemetry', [CompletionResultType]::ParameterValue, 'Control anonymous telemetry (on/off/status)')
            [CompletionResult]::new('auth', 'auth', [CompletionResultType]::ParameterValue, 'Authenticate with tok0 cloud for team analytics')
            [CompletionResult]::new('cloud', 'cloud', [CompletionResultType]::ParameterValue, 'View team dashboard (requires auth)')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completions')
            [CompletionResult]::new('trust', 'trust', [CompletionResultType]::ParameterValue, 'Trust a project directory for local filter rules')
            [CompletionResult]::new('untrust', 'untrust', [CompletionResultType]::ParameterValue, 'Remove trust from a project directory')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Verify hook file integrity')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'tok0;help;stats' {
            break
        }
        'tok0;help;git' {
            break
        }
        'tok0;help;read' {
            break
        }
        'tok0;help;ls' {
            break
        }
        'tok0;help;grep' {
            break
        }
        'tok0;help;find' {
            break
        }
        'tok0;help;diff' {
            break
        }
        'tok0;help;smart' {
            break
        }
        'tok0;help;proxy' {
            break
        }
        'tok0;help;status' {
            break
        }
        'tok0;help;doctor' {
            break
        }
        'tok0;help;rule' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all active rules (builtin + extensions + project-local)')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show full config for a single rule by name')
            [CompletionResult]::new('test', 'test', [CompletionResultType]::ParameterValue, 'Apply a rule to stdin and print the before/after savings')
            break
        }
        'tok0;help;rule;list' {
            break
        }
        'tok0;help;rule;show' {
            break
        }
        'tok0;help;rule;test' {
            break
        }
        'tok0;help;init' {
            break
        }
        'tok0;help;update' {
            break
        }
        'tok0;help;discover' {
            break
        }
        'tok0;help;rewrite' {
            break
        }
        'tok0;help;profile' {
            break
        }
        'tok0;help;ext' {
            [CompletionResult]::new('install', 'install', [CompletionResultType]::ParameterValue, 'Install extension from a git URL')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List installed extensions')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove an installed extension')
            break
        }
        'tok0;help;ext;install' {
            break
        }
        'tok0;help;ext;list' {
            break
        }
        'tok0;help;ext;remove' {
            break
        }
        'tok0;help;telemetry' {
            [CompletionResult]::new('on', 'on', [CompletionResultType]::ParameterValue, 'Enable anonymous telemetry')
            [CompletionResult]::new('off', 'off', [CompletionResultType]::ParameterValue, 'Disable anonymous telemetry')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show current telemetry status')
            break
        }
        'tok0;help;telemetry;on' {
            break
        }
        'tok0;help;telemetry;off' {
            break
        }
        'tok0;help;telemetry;status' {
            break
        }
        'tok0;help;auth' {
            [CompletionResult]::new('login', 'login', [CompletionResultType]::ParameterValue, 'Login with API token')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show authentication status')
            [CompletionResult]::new('logout', 'logout', [CompletionResultType]::ParameterValue, 'Remove stored credentials and disable cloud reporting')
            break
        }
        'tok0;help;auth;login' {
            break
        }
        'tok0;help;auth;status' {
            break
        }
        'tok0;help;auth;logout' {
            break
        }
        'tok0;help;cloud' {
            [CompletionResult]::new('team', 'team', [CompletionResultType]::ParameterValue, 'Show team savings summary')
            break
        }
        'tok0;help;cloud;team' {
            break
        }
        'tok0;help;completions' {
            break
        }
        'tok0;help;trust' {
            break
        }
        'tok0;help;untrust' {
            break
        }
        'tok0;help;verify' {
            break
        }
        'tok0;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
