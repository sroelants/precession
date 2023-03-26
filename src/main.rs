use std::path::PathBuf;
use serde::Deserialize;
use std::process::Command;
use eyre::Result;

const INITIAL_WINDOW: &str = "999";

fn main() -> Result<()> {
    let file_name = "./session.yml";
    let input = std::fs::read_to_string(file_name).expect("File not found");
    let session: Session = serde_yaml::from_str(&input).expect("Couldn't deserialize yaml");
    session.render()?;

    Ok(())
}

#[derive(Deserialize, Debug)]
struct Session {
    name: String,
    root: Option<PathBuf>,
    #[serde(default)]
    windows: Vec<Window>
}

#[derive(Deserialize, Debug, Default)]
struct Window {
    name: Option<String>,
    #[serde(default)]
    layout: Layout,
    root: Option<PathBuf>,
    cmd: Option<String>,
    #[serde(default)]
    panes: Option<Vec<Pane>>
}

impl Window {
    fn render(&self) -> eyre::Result<()> {
        self.create()?;

        // TODO: Validation: Either a command _or_ panes, never both!
        if let Some(command) = &self.cmd {
            self.run_cmd(&command)?;
        }

        if let Some(panes) = &self.panes {
            for (i, pane) in panes.iter().enumerate() {
                if i > 0 { 
                    pane.create()?;
                };

                pane.render()?;
            }
        }

        Command::new("tmux")
            .args(["select-layout", &self.layout.to_string()])
            .spawn()?
            .wait()?;

        Ok(())
    }

    fn create(&self) -> eyre::Result<()> {
        let mut create_cmd = Command::new("tmux");
        create_cmd.arg("new-window");

        if let Some(name) = &self.name {
            create_cmd.args(["-n", &format!("{name}")]);
        };

        if let Some(root) = &self.root {
            create_cmd.args(["-c", &format!("{}", root.to_string_lossy())]);
        };

        create_cmd.spawn()?.wait()?;

        Ok(())
    }

    fn run_cmd(&self, cmd: &str) -> eyre::Result<()> {
        Command::new("tmux")
            .args(["send-keys", cmd, "Enter"])
            .spawn()?
            .wait()?;

        Ok(())
    }
}

#[derive(Deserialize, Debug)]
#[serde(try_from = "String")]
enum Layout {
    Tiled,
    EvenHorizontal,
    EvenVertical,
    MainHorizontal,
    MainVertical,
}

impl TryFrom<String> for Layout {
    type Error = &'static str;

    fn try_from(str: String) -> Result<Self, Self::Error> {
        match str.as_str() {
            "tiled" => Ok(Layout::Tiled),
            "even-vertical" => Ok(Layout::EvenVertical),
            "even-horizontal" => Ok(Layout::EvenHorizontal),
            "main-vertical" => Ok(Layout::MainHorizontal),
            "main-horizontal" => Ok(Layout::MainHorizontal),
            _ => Err("Not a valid split direction"),
        }
    }
}

impl ToString for Layout {
    fn to_string(&self) -> String {
        match self {
            Layout::Tiled => "tiled",
            Layout::EvenHorizontal => "even-horizontal",
            Layout::EvenVertical => "even-vertical",
            Layout::MainHorizontal => "main-horizontal",
            Layout::MainVertical => "main-vertical",
        }.to_string()
    }
}

impl Default for Layout {
    fn default() -> Layout {
        Self::EvenHorizontal
    }
}

#[derive(Debug, Deserialize, Default)]
struct Pane(Option<String>);

impl Pane {
    fn render(&self) -> eyre::Result<()> {
        if let Some(command) = &self.0 {
            self.run_cmd(command)?;
        }

        Ok(())
    }

    fn create(&self) -> eyre::Result<()> {
        Command::new("tmux")
            .arg("split-window")
            .spawn()?
            .wait()?;
        Ok(())
    }

    fn run_cmd(&self, cmd: &str) -> eyre::Result<()> {
            Command::new("tmux")
                .args(["send-keys", cmd, "Enter"])
                .spawn()?
                .wait()?;
        Ok(())
    }

}

impl Session {
    fn render(&self) -> eyre::Result<()> {
        self.create()?;

        // Create all the windows
        for window in &self.windows {
            window.render()?;
        }

        self.finalize()?;
        Ok(())
    }

    fn create(&self) -> eyre::Result<()> {
        // Create the new session
        let mut create_cmd = Command::new("tmux");
        create_cmd.args(["new", "-d", "-s", &self.name, "-n", &INITIAL_WINDOW]);

        if let Some(root) = &self.root {
            create_cmd.args(["-c", &format!("{}", root.to_string_lossy())]);
        }

        create_cmd.spawn()?.wait()?;

        Ok(())
    }

    fn finalize(&self) -> eyre::Result<()> {
        // Remove the initial window and relabel the window
        Command::new("tmux")
           .args(["kill-window", "-t", &format!("{}:{INITIAL_WINDOW}", &self.name)])
           .spawn()?
           .wait()?;

        // Relabel windows
        Command::new("tmux")
            .args(["move-window", "-r"])
            .spawn()?
            .wait()?;

        // Attach to the new sesion (Should this be an option as well?
        Command::new("tmux")
            .args(["attach", "-t", &format!("{}:1", &self.name)])
            .spawn()?
            .wait()?;

        Ok(())
    }
}
