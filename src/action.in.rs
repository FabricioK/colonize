#[derive(Clone, Deserialize, Serialize)]
pub enum Action {
    Camera(CameraAction),
}
