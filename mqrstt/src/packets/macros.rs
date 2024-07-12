macro_rules! MqttAsyncRead {
    ( $name:ident, $id:expr, { $($fname:ident : $ftype:ty),* } ) => {
        #[derive(Codec, Debug, Eq, PartialEq, Clone)]
        pub struct $name{
           $(
                pub $fname: $ftype,
           )*
        }
        impl<T> crate::packets::mqtt_traits::MqttAsyncRead for $name where T: tokio::io::AsyncReadExt{
            fn name(&self) -> &'static str{
                stringify!($name)
            }

        }
    };
    ($name:ident, $id:expr) => {
        #[derive(Codec, Debug, Eq, PartialEq, Clone)]
        pub struct $name{}
        impl RequestTrait for $name{
            fn name(&self) -> &'static str{
                stringify!($name)
            }
            fn get_id(&self) -> u32 {
                return $id;
            }
        }
    }
}