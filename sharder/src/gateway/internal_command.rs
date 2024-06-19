use model::user::StatusUpdate;

pub enum InternalCommand {
    StatusUpdate { status: StatusUpdate },
    Shutdown,
}
