use tokio::sync::oneshot;

#[derive(Debug)]
pub enum Command {
    Search {
        query: String
    },
    Recommend {
        user_id: String
    }
}

pub struct AgentCommand {
    pub msg: Command,
    pub tx: oneshot::Sender<String>
}

