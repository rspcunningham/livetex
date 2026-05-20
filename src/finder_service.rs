use anyhow::{Context, Result, bail};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub const SERVICE_NAME: &str = "LiveTex Start Preview";
pub const WORKFLOW_DIR_NAME: &str = "LiveTex Start Preview.workflow";
const BUNDLE_ID: &str = "com.rspcunningham.livetex.startPreview";

#[derive(Debug, Clone)]
pub struct FinderServiceStatus {
    pub path: PathBuf,
    pub expected_binary: PathBuf,
    pub installed: bool,
    pub current: bool,
}

pub fn install() -> Result<PathBuf> {
    let workflow_dir = workflow_dir()?;
    let contents_dir = workflow_dir.join("Contents");
    let resources_dir = contents_dir.join("Resources");
    let live_tex = live_tex_binary()?;

    if workflow_dir.exists() {
        fs::remove_dir_all(&workflow_dir)
            .with_context(|| format!("Could not replace Finder Quick Action: {workflow_dir:?}"))?;
    }

    fs::create_dir_all(&resources_dir)
        .with_context(|| format!("Could not create Finder Quick Action: {resources_dir:?}"))?;
    fs::write(contents_dir.join("Info.plist"), info_plist())
        .with_context(|| format!("Could not write Finder Quick Action: {workflow_dir:?}"))?;
    fs::write(
        resources_dir.join("document.wflow"),
        document_workflow(&live_tex),
    )
    .with_context(|| format!("Could not write Finder Quick Action: {workflow_dir:?}"))?;

    refresh_services_menu();

    Ok(workflow_dir)
}

pub fn status() -> Result<FinderServiceStatus> {
    let path = workflow_dir()?;
    let expected_binary = live_tex_binary()?;
    let installed = path.join("Contents/Info.plist").is_file()
        && path.join("Contents/Resources/document.wflow").is_file();
    let current = installed && workflow_uses_binary(&path, &expected_binary)?;

    Ok(FinderServiceStatus {
        path,
        expected_binary,
        installed,
        current,
    })
}

fn workflow_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().with_context(|| "Could not determine home directory")?;
    Ok(home
        .join("Library")
        .join("Services")
        .join(WORKFLOW_DIR_NAME))
}

fn live_tex_binary() -> Result<PathBuf> {
    std::env::current_exe().with_context(|| "Could not determine LiveTex binary")
}

fn workflow_uses_binary(workflow_dir: &std::path::Path, binary: &std::path::Path) -> Result<bool> {
    let document_path = workflow_dir.join("Contents/Resources/document.wflow");
    let document = match fs::read_to_string(&document_path) {
        Ok(document) => document,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            bail!("Could not read Finder Quick Action: {document_path:?}: {error}");
        }
    };

    Ok(document.contains(&xml_escape(&shell_quote(&binary.to_string_lossy()))))
}

fn refresh_services_menu() {
    let _ = Command::new("/System/Library/CoreServices/pbs")
        .arg("-flush")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn info_plist() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en_US</string>
	<key>CFBundleIdentifier</key>
	<string>{BUNDLE_ID}</string>
	<key>CFBundleName</key>
	<string>{service_name}</string>
	<key>CFBundleShortVersionString</key>
	<string>1.0</string>
	<key>NSServices</key>
	<array>
		<dict>
			<key>NSMenuItem</key>
			<dict>
				<key>default</key>
				<string>{service_name}</string>
			</dict>
			<key>NSMessage</key>
			<string>runWorkflowAsService</string>
			<key>NSRequiredContext</key>
			<dict>
				<key>NSApplicationIdentifier</key>
				<string>com.apple.finder</string>
			</dict>
			<key>NSSendFileTypes</key>
			<array>
				<string>public.item</string>
			</array>
		</dict>
	</array>
</dict>
</plist>
"#,
        service_name = xml_escape(SERVICE_NAME)
    )
}

fn document_workflow(live_tex: &std::path::Path) -> String {
    let script = shell_script(live_tex);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>AMApplicationBuild</key>
	<string>346</string>
	<key>AMApplicationVersion</key>
	<string>2.3</string>
	<key>AMDocumentVersion</key>
	<string>2</string>
	<key>actions</key>
	<array>
		<dict>
			<key>action</key>
			<dict>
				<key>AMAccepts</key>
				<dict>
					<key>Container</key>
					<string>List</string>
					<key>Optional</key>
					<true/>
					<key>Types</key>
					<array>
						<string>com.apple.cocoa.path</string>
					</array>
				</dict>
				<key>AMActionVersion</key>
				<string>2.0.3</string>
				<key>AMApplication</key>
				<array>
					<string>Automator</string>
				</array>
				<key>AMParameterProperties</key>
				<dict>
					<key>COMMAND_STRING</key>
					<dict/>
					<key>CheckedForUserDefaultShell</key>
					<dict/>
					<key>inputMethod</key>
					<dict/>
					<key>shell</key>
					<dict/>
					<key>source</key>
					<dict/>
				</dict>
				<key>AMProvides</key>
				<dict>
					<key>Container</key>
					<string>List</string>
					<key>Types</key>
					<array>
						<string>com.apple.cocoa.string</string>
					</array>
				</dict>
				<key>ActionBundlePath</key>
				<string>/System/Library/Automator/Run Shell Script.action</string>
				<key>ActionName</key>
				<string>Run Shell Script</string>
				<key>ActionParameters</key>
				<dict>
					<key>COMMAND_STRING</key>
					<string>{script}</string>
					<key>CheckedForUserDefaultShell</key>
					<true/>
					<key>inputMethod</key>
					<integer>0</integer>
					<key>shell</key>
					<string>/bin/sh</string>
					<key>source</key>
					<string></string>
				</dict>
				<key>BundleIdentifier</key>
				<string>com.apple.RunShellScript</string>
				<key>CFBundleVersion</key>
				<string>2.0.3</string>
				<key>CanShowSelectedItemsWhenRun</key>
				<false/>
				<key>CanShowWhenRun</key>
				<true/>
				<key>Category</key>
				<array>
					<string>AMCategoryUtilities</string>
				</array>
				<key>Class Name</key>
				<string>RunShellScriptAction</string>
				<key>InputUUID</key>
				<string>92D26F3B-8D95-4D8B-B7E9-41472857218E</string>
				<key>Keywords</key>
				<array>
					<string>Shell</string>
					<string>Script</string>
					<string>Command</string>
					<string>Run</string>
					<string>Unix</string>
				</array>
				<key>OutputUUID</key>
				<string>36D6FEAF-7C20-434F-B2BB-4501B5EF8B8B</string>
				<key>UUID</key>
				<string>1A0AA4E1-31B0-49DD-881B-147C1CBB20B0</string>
				<key>UnlocalizedApplications</key>
				<array>
					<string>Automator</string>
				</array>
				<key>arguments</key>
				<dict>
					<key>0</key>
					<dict>
						<key>default value</key>
						<integer>0</integer>
						<key>name</key>
						<string>inputMethod</string>
						<key>required</key>
						<string>0</string>
						<key>type</key>
						<string>0</string>
						<key>uuid</key>
						<string>0</string>
					</dict>
					<key>1</key>
					<dict>
						<key>default value</key>
						<string></string>
						<key>name</key>
						<string>source</string>
						<key>required</key>
						<string>0</string>
						<key>type</key>
						<string>0</string>
						<key>uuid</key>
						<string>1</string>
					</dict>
					<key>2</key>
					<dict>
						<key>default value</key>
						<integer>1</integer>
						<key>name</key>
						<string>CheckedForUserDefaultShell</string>
						<key>required</key>
						<string>0</string>
						<key>type</key>
						<string>0</string>
						<key>uuid</key>
						<string>2</string>
					</dict>
					<key>3</key>
					<dict>
						<key>default value</key>
						<string></string>
						<key>name</key>
						<string>COMMAND_STRING</string>
						<key>required</key>
						<string>0</string>
						<key>type</key>
						<string>0</string>
						<key>uuid</key>
						<string>3</string>
					</dict>
					<key>4</key>
					<dict>
						<key>default value</key>
						<string>/bin/sh</string>
						<key>name</key>
						<string>shell</string>
						<key>required</key>
						<string>0</string>
						<key>type</key>
						<string>0</string>
						<key>uuid</key>
						<string>4</string>
					</dict>
				</dict>
			</dict>
			<key>isViewVisible</key>
			<true/>
		</dict>
	</array>
	<key>connectors</key>
	<dict/>
	<key>workflowMetaData</key>
	<dict>
		<key>serviceApplicationBundleID</key>
		<string>com.apple.finder</string>
		<key>serviceApplicationPath</key>
		<string>/System/Library/CoreServices/Finder.app</string>
		<key>serviceInputTypeIdentifier</key>
		<string>com.apple.Automator.fileSystemObject</string>
		<key>serviceOutputTypeIdentifier</key>
		<string>com.apple.Automator.nothing</string>
		<key>serviceProcessesInput</key>
		<integer>1</integer>
		<key>workflowTypeIdentifier</key>
		<string>com.apple.Automator.servicesMenu</string>
	</dict>
</dict>
</plist>
"#,
        script = xml_escape(&script)
    )
}

fn shell_script(live_tex: &std::path::Path) -> String {
    format!(
        r#"PATH="/opt/homebrew/bin:/usr/local/bin:/Library/TeX/texbin:/usr/bin:/bin:/usr/sbin:/sbin:$PATH"
export PATH

status=0
while IFS= read -r file; do
  [ -n "$file" ] || continue
  case "$file" in
    *.tex) {live_tex} start "$file" || status=$? ;;
  esac
done
exit "$status"
"#,
        live_tex = shell_quote(&live_tex.to_string_lossy())
    )
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn shell_script_quotes_live_tex_binary() {
        let script = shell_script(Path::new("/tmp/Live Tex/bin/livetex"));

        assert!(script.contains("'/tmp/Live Tex/bin/livetex' start \"$file\""));
        assert!(script.contains("/opt/homebrew/bin"));
        assert!(script.contains("/Library/TeX/texbin"));
        assert!(script.contains("*.tex)"));
    }

    #[test]
    fn workflow_contains_finder_service_metadata() {
        let workflow = document_workflow(Path::new("/tmp/livetex"));

        assert!(workflow.contains("com.apple.Automator.servicesMenu"));
        assert!(workflow.contains("/System/Library/Automator/Run Shell Script.action"));
        assert!(workflow.contains("&apos;/tmp/livetex&apos; start &quot;$file&quot;"));
    }

    #[test]
    fn info_plist_declares_finder_file_service() {
        let plist = info_plist();

        assert!(plist.contains("runWorkflowAsService"));
        assert!(plist.contains("com.apple.finder"));
        assert!(plist.contains("public.item"));
    }
}
