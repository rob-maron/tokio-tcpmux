
#[macro_export]
macro_rules! unwrap_err_return {
    ($out:expr, $type:expr, $context:expr) => {
        match $out {
            Ok(res) => res,
            Err(err) => {
                let custom_error = $type(format!("{}: {}", $context, err));
                return Err(custom_error);
            }
        }
    };
    ($out:expr, $type:expr, $context:expr, $after:expr) => {
        match $out {
            Ok(res) => res,
            Err(err) => {
                $after;
                let custom_error = $type(format!("{}: {}", $context, err));
                return Err(custom_error);
            }
        }
    };
}

#[macro_export]
macro_rules! unwrap_err_continue {
    ($out:expr, $type:expr, $context:expr) => {
        match $out {
            Ok(res) => res,
            Err(err) => {
                let custom_error = $type(format!("{}: {}", $context, err));
                tracing::error!("{}", custom_error);
                continue;
            }
        }
    };
    ($out:expr, $type:expr, $context:expr, $after:expr) => {
        match $out {
            Ok(res) => res,
            Err(err) => {
                $after;
                let custom_error = $type(format!("{}: {}", $context, err));
                error!("{}", custom_error);
                continue;
            }
        }
    };
}

#[macro_export]
macro_rules! unwrap_opt_continue {
    ($out:expr, $type:expr, $context:expr) => {
        match $out {
            Some(some) => some,
            None => {
                let custom_error = $type($context.to_string());
                tracing::error!("{}", custom_error);
                continue;
            }
        }
    };
    ($out:expr, $type:expr, $context:expr, $after:expr) => {
        match $out {
            Some(some) => some,
            None => {
                $after;
                let custom_error = $type($context.to_string());
                tracing::error!("{}", custom_error);
                continue;
            }
        }
    };
}

#[macro_export]
macro_rules! unwrap_opt_return {
    ($out:expr, $type:expr, $context:expr) => {
        match $out {
            Some(some) => some,
            None => {
                let custom_error = $type($context.to_string());
                return Err(custom_error);
            }
        }
    };
    ($out:expr, $type:expr, $context:expr, $after:expr) => {
        match $out {
            Some(some) => some,
            None => {
                $after;
                let custom_error = $type($context.to_string());
                return Err(custom_error);
            }
        }
    };
}

#[macro_export]
macro_rules! unwrap_opt {
    ($out:expr, $type:expr, $context:expr) => {
        match $out {
            Some(some) => some,
            None => {
                let custom_error = $type($context.to_string());
                tracing::error!("{}", custom_error);
                return;
            }
        }
    };
    ($out:expr, $type:expr, $context:expr, $after:expr) => {
        match $out {
            Some(some) => some,
            None => {
                $after;
                let custom_error = $type($context.to_string());
                error!("{}", custom_error);
                return;
            }
        }
    };
}

#[macro_export]
macro_rules! unwrap_err {
    ($out:expr, $type:expr, $context:expr) => {
        match $out {
            Ok(res) => res,
            Err(err) => {
                let custom_error = $type(format!("{}: {}", $context, err));
                tracing::error!("{}", custom_error);
                return;
            }
        }
    };
    ($out:expr, $type:expr, $context:expr, $after:expr) => {
        match $out {
            Ok(res) => res,
            Err(err) => {
                $after;
                let custom_error = $type(format!("{}: {}", $context, err));
                error!("{}", custom_error);
                return;
            }
        }
    };
}
