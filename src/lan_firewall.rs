use std::process::{Command, Stdio};
use std::os::windows::process::CommandExt;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;

const NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Debug, Default, Deserialize)]
struct Rule {
    name: String, display: String, program: String,
    enabled: bool, action: String, direction: String,
    source: String, store: String, profile: String,
    protocol: String, ports: Vec<String>, remote: Vec<String>,
}

fn quote(value: &str) -> String { format!("'{}'",value.replace('\'',"''")) }
fn encoded(script: &str) -> String { STANDARD.encode(script.encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<_>>()) }
fn run_powershell(script: &str) -> Result<std::process::Output,String> {
    Command::new("powershell.exe").args(["-NoProfile","-NonInteractive","-EncodedCommand",&encoded(script)])
        .creation_flags(NO_WINDOW).stdin(Stdio::null()).output().map_err(|error| error.to_string())
}

const READ_RULES: &str = r#"
$rules = @(Get-NetFirewallApplicationFilter -PolicyStore ActiveStore -Program $program -ErrorAction SilentlyContinue | Get-NetFirewallRule | ForEach-Object {
 $r=$_; $app=$r|Get-NetFirewallApplicationFilter; if ($app.Program -ine $program) { return }; $p=$r|Get-NetFirewallPortFilter; $a=$r|Get-NetFirewallAddressFilter;
 [pscustomobject]@{name=$r.Name;display=$r.DisplayName;program=$app.Program;enabled=($r.Enabled -eq 'True');action=$r.Action.ToString();direction=$r.Direction.ToString();source=$r.PolicyStoreSourceType.ToString();store=$r.PolicyStoreSource;profile=$r.Profile.ToString();protocol=$p.Protocol;ports=@($p.LocalPort);remote=@($a.RemoteAddress)}
});
"#;

fn read_rules(program: &str) -> Result<Vec<Rule>,String> {
    let script=format!("$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); $program={}; {} ConvertTo-Json -InputObject @($rules) -Depth 4 -Compress",quote(program),READ_RULES);
    let output=run_powershell(&script)?;
    if !output.status.success() { return Err("无法读取 Windows 防火墙有效规则。".into()); }
    serde_json::from_slice(&output.stdout).map_err(|_| "Windows 防火墙返回了无法识别的规则。".into())
}

fn protocol_matches(value: &str, expected: &str) -> bool {
    value.eq_ignore_ascii_case(expected) || value == if expected=="TCP" { "6" } else { "17" }
}
fn port_covers(ports: &[String], port: u16) -> bool {
    ports.iter().any(|value| value.eq_ignore_ascii_case("Any") || value.parse::<u16>().ok()==Some(port)
        || value.split_once('-').is_some_and(|(first,last)| first.parse::<u16>().ok().zip(last.parse::<u16>().ok()).is_some_and(|(first,last)| (first..=last).contains(&port))))
}
fn owned_allow(rule: &Rule) -> bool {
    let parts=rule.display.split_whitespace().collect::<Vec<_>>();
    rule.source=="Local" && rule.store=="PersistentStore" && (parts.len()==6 || parts.len()==7)
        && parts[0]=="ZSClip" && parts[1]=="LAN"
        && ((parts[2]=="Sync" && parts[3]=="TCP") || (parts[2]=="Discovery" && parts[3]=="UDP"))
        && parts[4].parse::<u16>().is_ok() && parts[5].len()==8 && parts[5].chars().all(|ch|ch.is_ascii_hexdigit())
        && (parts.len()==6 || parts[6]=="LocalSubnetV2")
}
fn repairable_user_block(rule: &Rule) -> bool {
    rule.source=="Local" && rule.store=="PersistentStore"
        && (rule.name.starts_with("TCP Query User{") || rule.name.starts_with("UDP Query User{"))
}
fn scoped_allow(rule: &Rule, protocol: &str, port: u16) -> bool {
    rule.enabled && rule.direction=="Inbound" && rule.action=="Allow"
        && protocol_matches(&rule.protocol,protocol) && rule.ports==[port.to_string()]
        && rule.remote==["LocalSubnet"] && rule.profile=="Any"
}
fn verify_rules(rules: &[Rule], program: &str, tcp: u16, udp: u16) -> Result<(),String> {
    let matching=rules.iter().filter(|rule|rule.program.eq_ignore_ascii_case(program) && rule.enabled && rule.direction=="Inbound").collect::<Vec<_>>();
    let blocks=matching.iter().filter(|rule|rule.action=="Block" && ((protocol_matches(&rule.protocol,"TCP") && port_covers(&rule.ports,tcp))
        || (protocol_matches(&rule.protocol,"UDP") && port_covers(&rule.ports,udp)) || rule.protocol.eq_ignore_ascii_case("Any"))).collect::<Vec<_>>();
    if blocks.iter().any(|rule|!repairable_user_block(rule)) {
        return Err("存在手动或管理策略的入站阻止规则；自动修复不会修改这些规则。".into());
    }
    if !blocks.is_empty() && matching.iter().any(|rule|rule.action=="Allow" && !owned_allow(rule)
        && !(rule.remote==["LocalSubnet"] && ((protocol_matches(&rule.protocol,"TCP") && rule.ports==[tcp.to_string()])
            || (protocol_matches(&rule.protocol,"UDP") && rule.ports==[udp.to_string()])))) {
        return Err("检测到其他宽范围允许规则，解除拒绝前需在 Windows 防火墙检查。".into());
    }
    if !blocks.is_empty() { return Err("发现 Windows 为本程序保存的拒绝连接规则；点击“刷新 / 修复连接”解除冲突。".into()); }
    if !matching.iter().any(|rule|scoped_allow(rule,"TCP",tcp)) || !matching.iter().any(|rule|scoped_allow(rule,"UDP",udp)) {
        return Err("本程序的局域网入站权限不完整，请点击“刷新 / 修复连接”。".into());
    }
    Ok(())
}

fn program() -> Result<String,String> { std::env::current_exe().map(|path|path.to_string_lossy().to_string()).map_err(|error|error.to_string()) }
pub(crate) fn verify(tcp: u16, udp: u16) -> Result<(),String> {
    let exe=program()?;
    verify_rules(&read_rules(&exe)?,&exe,tcp,udp)
}
pub(crate) fn ensure(tcp: u16, udp: u16) -> Result<(),String> {
    let exe=program()?; let rules=read_rules(&exe)?;
    for (protocol,port,kind) in [("TCP",tcp,"Sync"),("UDP",udp,"Discovery")] {
        if rules.iter().any(|rule|scoped_allow(rule,protocol,port)) { continue; }
        let name=format!("ZSClip LAN {kind} {protocol} {port} {} LocalSubnetV2",crate::lan_sync_core::lan_hash_string(&exe).chars().take(8).collect::<String>());
        let status=Command::new("netsh.exe").args(["advfirewall","firewall","add","rule",&format!("name={name}"),"dir=in","action=allow",&format!("program={exe}"),&format!("protocol={protocol}"),&format!("localport={port}"),"profile=any","remoteip=localsubnet","enable=yes"])
            .creation_flags(NO_WINDOW).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().map_err(|error|error.to_string())?;
        if !status.success() { return Err("需要 Windows 管理员确认以允许局域网连接。".into()); }
    }
    verify(tcp,udp)
}

fn repair_script(exe: &str,tcp: u16,udp: u16,backup: &str) -> String {
    let key=crate::lan_sync_core::lan_hash_string(exe).chars().take(8).collect::<String>();
    let mut script=format!("$ErrorActionPreference='Stop'; $program={}; $tcp={tcp}; $udp={udp}; $backup={};\n{}",quote(exe),quote(backup),READ_RULES);
    script.push_str(r#"
function Test-Owned($r) { $r.source -eq 'Local' -and $r.store -eq 'PersistentStore' -and $r.display -match '^ZSClip LAN (Sync TCP|Discovery UDP) [0-9]+ [a-fA-F0-9]{8}( LocalSubnetV2)?$' }
function Test-UserBlock($r) { $r.source -eq 'Local' -and $r.store -eq 'PersistentStore' -and $r.name -match '^(TCP|UDP) Query User\{' }
function Test-Port($ports,$wanted) { foreach($p in $ports) { if ($p -eq 'Any' -or $p -eq "$wanted") { return $true }; if ($p -match '^(\d+)-(\d+)$' -and $wanted -ge [int]$Matches[1] -and $wanted -le [int]$Matches[2]) { return $true } }; return $false }
$blocks=@($rules|Where-Object {$_.enabled -and $_.direction -eq 'Inbound' -and $_.action -eq 'Block' -and ((($_.protocol -in @('TCP','6')) -and (Test-Port $_.ports $tcp)) -or (($_.protocol -in @('UDP','17')) -and (Test-Port $_.ports $udp)) -or $_.protocol -eq 'Any')});
if (@($blocks|Where-Object {-not(Test-UserBlock $_)}).Count) { throw 'Managed or manually configured block rules are preserved.' }
$broadAllow=@($rules|Where-Object {$_.enabled -and $_.direction -eq 'Inbound' -and $_.action -eq 'Allow' -and -not(Test-Owned $_) -and -not ($_.remote.Count -eq 1 -and $_.remote[0] -eq 'LocalSubnet' -and $_.ports.Count -eq 1 -and ((($_.protocol -in @('TCP','6')) -and $_.ports[0] -eq "$tcp") -or (($_.protocol -in @('UDP','17')) -and $_.ports[0] -eq "$udp")))});
if ($blocks.Count -and $broadAllow.Count) { throw 'Existing broad custom allow rules require review before removing a block.' }
New-Item -ItemType Directory -Force -Path ([IO.Path]::GetDirectoryName($backup)) | Out-Null;
ConvertTo-Json -InputObject @($rules) -Depth 5 | Set-Content -LiteralPath $backup -Encoding UTF8;
foreach($r in $rules|Where-Object {$_.enabled -and $_.direction -eq 'Inbound' -and $_.action -eq 'Allow' -and (Test-Owned $_)}) {
 if ((($r.protocol -in @('TCP','6')) -and $r.ports.Count -eq 1 -and $r.ports[0] -eq "$tcp") -or (($r.protocol -in @('UDP','17')) -and $r.ports.Count -eq 1 -and $r.ports[0] -eq "$udp")) { Set-NetFirewallRule -PolicyStore PersistentStore -Name $r.name -RemoteAddress LocalSubnet }
 else { Set-NetFirewallRule -PolicyStore PersistentStore -Name $r.name -Enabled False }
}
foreach($r in $blocks) { Set-NetFirewallRule -PolicyStore PersistentStore -Name $r.name -Enabled False }
"#);
    for (protocol,port,kind) in [("TCP",tcp,"Sync"),("UDP",udp,"Discovery")] {
        let name=format!("ZSClip LAN {kind} {protocol} {port} {key} LocalSubnetV2");
        script.push_str(&format!("if (-not @($rules|Where-Object {{$_.enabled -and $_.action -eq 'Allow' -and $_.direction -eq 'Inbound' -and $_.profile -eq 'Any' -and $_.protocol -eq '{protocol}' -and $_.ports.Count -eq 1 -and $_.ports[0] -eq '{port}' -and $_.remote.Count -eq 1 -and $_.remote[0] -eq 'LocalSubnet'}}).Count) {{ & ($env:SystemRoot + '\\System32\\netsh.exe') "));
        let args=["advfirewall".to_string(),"firewall".to_string(),"add".to_string(),"rule".to_string(),format!("name={name}"),"dir=in".into(),"action=allow".into(),format!("program={exe}"),format!("protocol={protocol}"),format!("localport={port}"),"profile=any".into(),"remoteip=localsubnet".into(),"enable=yes".into()];
        script.push_str(&args.iter().map(|arg|quote(arg)).collect::<Vec<_>>().join(" "));
        script.push_str("; if ($LASTEXITCODE -ne 0) { exit 1 } };\n");
    }
    script.push_str("exit 0"); script
}

pub(crate) fn repair_elevated(tcp: u16,udp: u16) -> Result<(),String> {
    use crate::platform::string::to_wide;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess,WaitForSingleObject};
    use windows_sys::Win32::UI::Shell::{ShellExecuteExW,SHELLEXECUTEINFOW,SEE_MASK_NOCLOSEPROCESS};
    let exe=program()?;
    let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
    let backup=crate::app::data_dir().join("firewall-backups").join(format!("lan-{stamp}.json"));
    let script=encoded(&repair_script(&exe,tcp,udp,&backup.to_string_lossy()));
    let verb=to_wide("runas");
    let powershell=to_wide(&format!("{}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",std::env::var("SystemRoot").unwrap_or_else(|_|"C:\\Windows".into())));
    let args=to_wide(&format!("-NoProfile -NonInteractive -EncodedCommand {script}"));
    unsafe {
        let mut info:SHELLEXECUTEINFOW=std::mem::zeroed(); info.cbSize=std::mem::size_of::<SHELLEXECUTEINFOW>() as u32; info.fMask=SEE_MASK_NOCLOSEPROCESS;
        info.lpVerb=verb.as_ptr();info.lpFile=powershell.as_ptr();info.lpParameters=args.as_ptr();info.nShow=0;
        if ShellExecuteExW(&mut info)==0 { return Err(std::io::Error::last_os_error().to_string()); }
        if info.hProcess.is_null() { return Err("系统未返回防火墙修复进程。".into()); }
        let wait=WaitForSingleObject(info.hProcess,60_000);let mut code=1;let read=GetExitCodeProcess(info.hProcess,&mut code);CloseHandle(info.hProcess);
        if wait!=0 || read==0 || code!=0 { return Err("防火墙修复未完成；管理策略和其他程序规则均保持原样。".into()); }
    }
    verify(tcp,udp)
}

#[cfg(test)]
mod tests {
    use super::*;
    const EXE:&str=r"C:\Apps\ZSClip\剪贴板.exe";
    fn allow(protocol:&str,port:u16)->Rule { Rule { name:"allow".into(),display:format!("ZSClip LAN {} {protocol} {port} 1234abcd LocalSubnetV2",if protocol=="TCP"{"Sync"}else{"Discovery"}),program:EXE.into(),enabled:true,action:"Allow".into(),direction:"Inbound".into(),source:"Local".into(),store:"PersistentStore".into(),profile:"Any".into(),protocol:protocol.into(),ports:vec![port.to_string()],remote:vec!["LocalSubnet".into()] } }
    #[test] fn allow_does_not_hide_local_or_managed_block() {
        let mut rules=vec![allow("TCP",38473),allow("UDP",38472)]; assert!(verify_rules(&rules,EXE,38473,38472).is_ok());
        let mut block=allow("UDP",38472);block.name="UDP Query User{test}".into();block.action="Block".into();block.ports=vec!["Any".into()];block.profile="Public".into();
        rules.push(block);assert!(verify_rules(&rules,EXE,38473,38472).unwrap_err().contains("拒绝"));assert!(repairable_user_block(&rules[2]));
        let mut broad=allow("TCP",38473);broad.display="Custom permit".into();broad.remote=vec!["10.0.0.0/8".into()];rules.push(broad);
        assert!(verify_rules(&rules,EXE,38473,38472).unwrap_err().contains("其他宽范围"));rules.pop();
        rules[2].source="GroupPolicy".into();assert!(!repairable_user_block(&rules[2]));assert!(verify_rules(&rules,EXE,38473,38472).unwrap_err().contains("管理策略"));
        rules[2].program=r"C:\Other\other.exe".into();assert!(verify_rules(&rules,EXE,38473,38472).is_ok());
    }
    #[test] fn repair_scope_is_local_program_user_queries_and_subnet_only() {
        let script=repair_script(r"C:\Input's $folder\剪贴板.exe",38481,38472,r"C:\Data\backup.json");
        assert!(script.contains("$program='C:\\Input''s $folder\\剪贴板.exe'"));
        assert!(script.contains("$r.source -eq 'Local' -and $r.store -eq 'PersistentStore'"));
        assert!(script.contains("Query User\\{"));assert_eq!(script.matches("'remoteip=localsubnet'").count(),2);
        assert!(script.contains("$_.action -eq 'Allow' -and $_.direction -eq 'Inbound' -and $_.profile -eq 'Any'"));
        assert!(!script.contains("set allprofiles"));assert!(script.contains("Set-Content -LiteralPath $backup"));
        assert!(owned_allow(&allow("TCP",38473)));
        let parse=format!("$s=[Text.Encoding]::Unicode.GetString([Convert]::FromBase64String('{}'));$t=$null;$e=$null;[void][Management.Automation.Language.Parser]::ParseInput($s,[ref]$t,[ref]$e);if($e.Count){{exit 1}}",encoded(&script));
        assert!(run_powershell(&parse).unwrap().status.success());
    }
    #[test] fn outbound_allow_is_not_an_inbound_permission() {
        let mut rules=vec![allow("TCP",38473),allow("UDP",38472)];
        for rule in &mut rules { rule.direction="Outbound".into(); }
        assert!(verify_rules(&rules,EXE,38473,38472).is_err());
        assert!(!scoped_allow(&rules[0],"TCP",38473));
    }
}
