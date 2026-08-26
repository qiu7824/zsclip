$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$left = [int]$args[0]
$top = [int]$args[1]
$width = [int]$args[2]
$height = [int]$args[3]

[System.Windows.Forms.Application]::EnableVisualStyles()

$form = New-Object System.Windows.Forms.Form
$form.Text = "ZSClip 演示目标 - 编辑器"
$form.StartPosition = "Manual"
$form.Left = $left
$form.Top = $top
$form.Width = $width
$form.Height = $height
$form.TopMost = $true
$form.BackColor = [System.Drawing.Color]::White

$textBox = New-Object System.Windows.Forms.TextBox
$textBox.Multiline = $true
$textBox.AcceptsReturn = $true
$textBox.AcceptsTab = $true
$textBox.ScrollBars = "Vertical"
$textBox.BorderStyle = "None"
$textBox.Dock = "Fill"
$textBox.Font = New-Object System.Drawing.Font("Microsoft YaHei UI", 13)
$textBox.Margin = New-Object System.Windows.Forms.Padding(20)
$textBox.Text = "ZSClip desktop demo target`r`n`r`n"

$panel = New-Object System.Windows.Forms.Panel
$panel.Dock = "Fill"
$panel.Padding = New-Object System.Windows.Forms.Padding(28, 28, 28, 28)
$panel.BackColor = [System.Drawing.Color]::White
$panel.Controls.Add($textBox)
$form.Controls.Add($panel)

$form.Add_Shown({
  $form.Activate()
  $textBox.Focus()
  $textBox.SelectionStart = $textBox.TextLength
  $textBox.SelectionLength = 0
})

[System.Windows.Forms.Application]::Run($form)
