#[macro_export]
macro_rules! inject_services {
    ($manager:expr, $fn_name:expr, { $($var_name:ident : $service_type:ty),* $(,)? }) => {
        $(
            let $var_name = $manager.get::<$service_type>($fn_name).await.unwrap();
        )*
    };
}
