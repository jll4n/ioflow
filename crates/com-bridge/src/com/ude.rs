/// Vrais appels COM/UDE vers Control Expert.
/// Ce module n'est compilé que si la feature "com" est activée (build x86).
use shared::bridge::{BridgeResponse, BuildResult};
use uuid::Uuid;

pub fn open_project(_path: &str) -> BridgeResponse {
    // TODO: CoInitialize, CoCreateInstance(CLSID_UnityApplication), OpenProject
    BridgeResponse::Error {
        message: "UDE COM not yet implemented".into(),
    }
}

pub fn build(_job_id: Uuid) -> BridgeResponse {
    // TODO: IUnityProject::Build(), lire la liste d'erreurs/warnings
    BridgeResponse::Error {
        message: "UDE COM not yet implemented".into(),
    }
}

pub fn close_project() -> BridgeResponse {
    // TODO: IUnityProject::Close(), CoUninitialize
    BridgeResponse::Error {
        message: "UDE COM not yet implemented".into(),
    }
}
