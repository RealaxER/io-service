
#[derive(Debug, PartialEq,Clone)]
pub enum OtaErr {
    SetValueErr,
    SelectPinErr,
    SetDirectionErr,
    GetValueErr,
    HttpErr,
    MqttErr,
    TimoutErr,
    RepeatErr,
    OpenFileErr,
    ReadFileErr,
    ConvertTempErr,
}