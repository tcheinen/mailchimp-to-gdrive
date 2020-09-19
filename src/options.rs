use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about, author)]
pub struct Arguments {
    #[structopt(short, long, name = "drive-id", index = 1)]
    pub drive_id: String,
    #[structopt(short, long, default_value = "clientsecret.json")]
    pub secret: String,
    #[structopt(short, long, default_value = "3030")]
    pub port: u16,
}
