#requires -version 3.0

#{# template engine Tera #}

#------------------------------------------------------------------------------
# 01) Runtime Setup
#------------------------------------------------------------------------------
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RUN_DIRECTORY = $PWD.Path
$_QUIET = {{ quiet | escape_shell }}
$_FORCE = {{ force | escape_shell }}
$_CANONICAL_BINARY_NAME = {{ app | escape_shell }}

$_E_GENERIC_ERROR = 10

#------------------------------------------------------------------------------
# 02) Temporary Workspace and Exit Cleanup
#------------------------------------------------------------------------------
$_TMPDIR = New-TemporaryFile | ForEach-Object {
    Remove-Item $_
    New-Item -ItemType Directory -Path $_
}
Set-Location $_TMPDIR.FullName

$cleanup = {
    if (Test-Path $_TMPDIR) {
        Write-LogMessage "DEBUG" "Removing $_TMPDIR"
        Remove-Item $_TMPDIR -Recurse -Force -ErrorAction SilentlyContinue
    }
    Set-Location $RUN_DIRECTORY
}

Register-EngineEvent PowerShell.Exiting -Action $cleanup | Out-Null
trap { & $cleanup; break }

#------------------------------------------------------------------------------
# 03) Log Level Configuration
#------------------------------------------------------------------------------
$INSTALL_LOG_LEVEL = {{ log_level | escape_shell }}
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

#------------------------------------------------------------------------------
# 04) Logging Helper
#------------------------------------------------------------------------------
function Write-LogMessage {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Level,
        [Parameter(Mandatory = $true)]
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

#------------------------------------------------------------------------------
# 05) Interactive Choice Prompt
#------------------------------------------------------------------------------
function Get-UserChoice {
    param(
        [Parameter(Mandatory = $true)]
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

#------------------------------------------------------------------------------
# 06) Download Helper
#------------------------------------------------------------------------------
function Get-WebContent {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Url,
        [string]$OutFile
    )

    try {
        if ($OutFile) {
            Invoke-WebRequest -Uri $Url -OutFile $OutFile -UseBasicParsing -ErrorAction Stop
            return $true
        } else {
            return (Invoke-WebRequest -Uri $Url -UseBasicParsing -ErrorAction Stop).Content
        }
    }
    catch {
        Write-LogMessage "ERROR" "Failed to download from $Url`: $_"
        if ($OutFile) {
            return $false
        }
        return $null
    }
}

#------------------------------------------------------------------------------
# 07) Rendered Asset Arrays
#------------------------------------------------------------------------------
{% if (assets | length > 0) %}
$_urls = @({% for asset in assets %}"{{ asset.url | escape_shell }}"{% if not loop.last %}, {% endif %}{% endfor %})
$_filenames = @({% for asset in assets %}"{{ asset.name | escape_shell }}"{% if not loop.last %}, {% endif %}{% endfor %})
$_filetypes = @({% for asset in assets %}"{{ asset.filetype | escape_shell }}"{% if not loop.last %}, {% endif %}{% endfor %})
$_printables = @({% for asset in assets %}"{{ asset.name ~ " (" ~ asset.filetype ~ ")" | escape_shell }}"{% if not loop.last %}, {% endif %}{% endfor %})

#------------------------------------------------------------------------------
# 08) Asset Selection
#------------------------------------------------------------------------------
Write-Host "Please select one of the following:"
$choice = Get-UserChoice -Choices $_printables -AllowQuit

#------------------------------------------------------------------------------
# 09) Selection Validation
#------------------------------------------------------------------------------
if ($choice -eq "q" -or $choice -eq "n") {
    exit 0
}

if ($choice -lt 0 -or $choice -ge $_urls.Count) {
    Write-LogMessage "FATAL" "invalid choice: $choice"
}

#------------------------------------------------------------------------------
# 10) Download and Install Dispatch
#------------------------------------------------------------------------------
Write-Host "Downloading from $($_urls[$choice]) to $_TMPDIR"
$_type = $_filetypes[$choice]

switch ($_type) {
    "binary" {
        $filename = $_filenames[$choice]
        $saved_file = Join-Path $_TMPDIR.FullName $filename
        if (-not (Get-WebContent -Url $_urls[$choice] -OutFile $saved_file)) {
            Write-LogMessage "FATAL" "failed downloading binary asset"
        }

        if ([string]::IsNullOrWhiteSpace($_CANONICAL_BINARY_NAME)) {
            $binary_name = Read-Host "enter alternate binary name (default: $filename)"
            if ([string]::IsNullOrWhiteSpace($binary_name)) {
                $binary_name = $filename
            }
        }
        else {
            $binary_name = $_CANONICAL_BINARY_NAME
        }

        $default_bin_dir = Join-Path $RUN_DIRECTORY "bin"
        $binary_dir = Read-Host "enter alternate binary directory (default: $default_bin_dir)"
        if ([string]::IsNullOrWhiteSpace($binary_dir)) {
            $binary_dir = $default_bin_dir
        }
        if (-not (Test-Path $binary_dir)) {
            New-Item -ItemType Directory -Path $binary_dir -Force | Out-Null
        }

        $dest_path = Join-Path $binary_dir $binary_name
        Copy-Item $saved_file $dest_path -Force
        Write-Host "Installed $binary_name to $dest_path"
    }
    "deb installer" {
        Write-LogMessage "FATAL" "deb installer is not supported on Windows"
    }
    "rpm installer" {
        Write-LogMessage "FATAL" "rpm installer is not supported on Windows"
    }
    "pkg installer" {
        Write-LogMessage "FATAL" "pkg installer is not supported on Windows"
    }
    "msi installer" {
        $filename = $_filenames[$choice]
        $saved_file = Join-Path $_TMPDIR.FullName $filename
        if (-not (Get-WebContent -Url $_urls[$choice] -OutFile $saved_file)) {
            Write-LogMessage "FATAL" "failed downloading msi installer"
        }
        Write-Host "Launching MSI installer..."
        Start-Process msiexec.exe -ArgumentList "/i `"$saved_file`"" -Wait
    }
    "exe installer" {
        $filename = $_filenames[$choice]
        $saved_file = Join-Path $_TMPDIR.FullName $filename
        if (-not (Get-WebContent -Url $_urls[$choice] -OutFile $saved_file)) {
            Write-LogMessage "FATAL" "failed downloading exe installer"
        }
        Write-Host "Launching EXE installer..."
        Start-Process -FilePath $saved_file -Wait
    }
    "tar.gz" {
        $filename = $_filenames[$choice]

        # Download and extract tar.gz
        $archive_path = Join-Path $_TMPDIR.FullName $filename
        if (-not (Get-WebContent -Url $_urls[$choice] -OutFile $archive_path)) {
            Write-LogMessage "FATAL" "failed downloading tar.gz archive"
        }

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
            if ($_.Extension -eq ".exe" -or $_.Extension -eq "" -or $_.Name -notmatch '\.') {
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
                if (-not (Test-Path $default_bin_dir)) {
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
#------------------------------------------------------------------------------
# 11) No Assets Available
#------------------------------------------------------------------------------
Write-LogMessage "FATAL" "no assets found"
{% endif %}

# cleanup execution for non-engine-exit completion paths
& $cleanup
