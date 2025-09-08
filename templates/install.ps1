#requires -version 3.0

#{# template engine Tera #}

# Set strict mode and error action preference
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Store original directory and template variables
$RUN_DIRECTORY = $PWD.Path
$_QUIET = {{ quiet | escape_shell }}
$_FORCE = {{ force | escape_shell }}
$_CANONICAL_BINARY_NAME = {{ app | escape_shell }}

# Error codes
$_E_GENERIC_ERROR = 10

# Create temporary directory
$_TMPDIR = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }
Set-Location $_TMPDIR.FullName

# Cleanup function
$cleanup = {
    if (Test-Path $_TMPDIR) {
        Write-LogMessage "DEBUG" "Removing $_TMPDIR"
        Set-Location $RUN_DIRECTORY
        Remove-Item $_TMPDIR -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# Register cleanup on exit
Register-EngineEvent PowerShell.Exiting -Action $cleanup | Out-Null
trap { & $cleanup; break }

# Log level configuration
$INSTALL_LOG_LEVEL = "{{ log_level | escape_shell }}"
switch ($INSTALL_LOG_LEVEL) {
    "TRACE" {
        $INSTALL_LOG_LEVEL = 0
        $VerbosePreference = "Continue"
        $DebugPreference = "Continue"
    }
    "DEBUG" { $INSTALL_LOG_LEVEL = 1 }
    "INFO" { $INSTALL_LOG_LEVEL = 2 }
    "WARN" { $INSTALL_LOG_LEVEL = 3 }
    "ERROR" { $INSTALL_LOG_LEVEL = 4 }
    "FATAL" { $INSTALL_LOG_LEVEL = 5 }
    default {
        $INSTALL_LOG_LEVEL = 2
        Write-Error "invalid log level: $INSTALL_LOG_LEVEL, using INFO"
    }
}

# Logging function
function Write-LogMessage {
    param(
        [Parameter(Mandatory=$true)]
        [string]$Level,
        [Parameter(Mandatory=$true)]
        [string]$Message
    )

    $levelNum = switch ($Level) {
        "DEBUG" { 1 }
        "INFO" { 2 }
        "WARN" { 3 }
        "ERROR" { 4 }
        "FATAL" { 5 }
        default { return }
    }

    if ($levelNum -ge $INSTALL_LOG_LEVEL) {
        Write-Host "$Level`: $Message" -ForegroundColor $(
            switch ($Level) {
                "DEBUG" { "Gray" }
                "INFO" { "White" }
                "WARN" { "Yellow" }
                "ERROR" { "Red" }
                "FATAL" { "DarkRed" }
            }
        )

        if ($Level -eq "FATAL") {
            exit 100
        }
    }
}

# Choice selection function
function Get-UserChoice {
    param(
        [Parameter(Mandatory=$true)]
        [string[]]$Choices,
        [switch]$AllowNone,
        [switch]$AllowQuit
    )

    if ($Choices.Count -eq 0) {
        Write-LogMessage "FATAL" "no choices provided"
        exit 1
    }

    $idx = 1
    foreach ($choice in $Choices) {
        Write-Host "`t$idx)`t$choice"
        $idx++
    }

    if ($AllowNone) {
        Write-Host "`tn)`tnone"
    }
    if ($AllowQuit) {
        Write-Host "`tq)`tquit"
    }

    do {
        $userInput = Read-Host "Enter choice"

        if ($AllowQuit -and ($userInput -eq "q" -or $userInput -eq "quit")) {
            return "q"
        }
        if ($AllowNone -and ($userInput -eq "n" -or $userInput -eq "none")) {
            return "n"
        }

        $choiceNum = 0
        if ([int]::TryParse($userInput, [ref]$choiceNum)) {
            if ($choiceNum -ge 1 -and $choiceNum -le $Choices.Count) {
                return ($choiceNum - 1)
            }
        }

        Write-Host "Invalid choice. Please try again." -ForegroundColor Red
    } while ($true)
}

# Download function
function Get-WebContent {
    param(
        [Parameter(Mandatory=$true)]
        [string]$Url,
        [string]$OutFile
    )

    try {
        if ($OutFile) {
            Invoke-WebRequest -Uri $Url -OutFile $OutFile -UseBasicParsing -ErrorAction Stop
        } else {
            return (Invoke-WebRequest -Uri $Url -UseBasicParsing -ErrorAction Stop).Content
        }
    }
    catch {
        Write-LogMessage "ERROR" "Failed to download from $Url`: $_"
        return $null
    }
}

{% if (assets | length > 0) %}
# Asset arrays
$_urls = @({% for asset in assets %}"{{ asset.url | escape_shell }}"{% if not loop.last %}, {% endif %}{% endfor %})
$_filenames = @({% for asset in assets %}"{{ asset.name | escape_shell }}"{% if not loop.last %}, {% endif %}{% endfor %})
$_filetypes = @({% for asset in assets %}"{{ asset.filetype | escape_shell }}"{% if not loop.last %}, {% endif %}{% endfor %})
$_printables = @({% for asset in assets %}"{{ asset.name ~ " (" ~ asset.filetype ~ ")" | escape_shell }}"{% if not loop.last %}, {% endif %}{% endfor %})

Write-Host "Please select one of the following:"
$choice = Get-UserChoice -Choices $_printables -AllowQuit

if ($choice -eq "q" -or $choice -eq "n") {
    exit 0
}

if ($choice -lt 0 -or $choice -ge $_urls.Count) {
    Write-LogMessage "FATAL" "invalid choice: $choice"
}

Write-Host "Downloading from $($_urls[$choice]) to $_TMPDIR"
$_type = $_filetypes[$choice]

switch ($_type) {
    "tar.gz" {
        $filename = $_filenames[$choice]

        # Download and extract tar.gz
        $archive_path = Join-Path $_TMPDIR.FullName $filename
        Get-WebContent -Url $_urls[$choice] -OutFile $archive_path

        # Extract using tar (available in Windows 10 1803+) or 7-Zip if available
        if (Get-Command tar -ErrorAction SilentlyContinue) {
            tar -xzf $archive_path
        }
        elseif (Get-Command 7z -ErrorAction SilentlyContinue) {
            7z x $archive_path
            $tar_file = $archive_path -replace '\.gz$', ''
            if (Test-Path $tar_file) {
                7z x $tar_file
                Remove-Item $tar_file -Force
            }
        }
        else {
            Write-LogMessage "FATAL" "No extraction tool found. Please install tar or 7-Zip"
        }

        # Find executable files
        $executable_files = @()
        Get-ChildItem -Recurse -File | ForEach-Object {
            if ($_.Extension -eq ".exe" -or $_.Extension -eq "" -or $_.Name -notmatch '\.' ) {
                $executable_files += $_.FullName
            }
        }

        if ($executable_files.Count -eq 0) {
            Write-LogMessage "FATAL" "no executable files found in archive"
        } else {
            $choices = Get-UserChoice -Choices $executable_files -AllowQuit

            if ($choices -ne "q") {
                $selected_file = $executable_files[$choices]
                $default_bin_dir = Join-Path $RUN_DIRECTORY "bin"
                if (!(Test-Path $default_bin_dir)) {
                    New-Item -ItemType Directory -Path $default_bin_dir -Force | Out-Null
                }

                $dest_name = Split-Path $selected_file -Leaf
                $dest_path = Join-Path $default_bin_dir $dest_name
                Copy-Item $selected_file $dest_path -Force
                Write-Host "Installed $dest_name to $dest_path"
            }
        }
    }
    default {
        Write-LogMessage "FATAL" "invalid filetype: $_type"
    }
}
{% else %}
Write-LogMessage "FATAL" "no assets found"
{% endif %}

# Cleanup
& $cleanup
