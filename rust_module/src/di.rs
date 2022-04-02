use shaku::module;

use crate::application::factory::FactoryImpl;
use crate::application::usecase::data::redirect::Redirect;
use crate::application::usecase::event::EventReceiveImpl;
use crate::application::usecase::General;
use crate::infra::RepositoryImpl;
use crate::utils::CallbackCallerImpl;
use crate::GlobalStateImpl;

module! {
    pub(crate) GeneralFactory {
        components = [FactoryImpl],
        providers = []
    }
}

module! {
    pub(crate) RepositoryModule {
        components = [RepositoryImpl, GlobalStateImpl],
        providers = []
    }
}

module! {
    pub(crate) GeneralService {
        components = [General, RepositoryImpl, GlobalStateImpl],
        providers = []
    }
}

module! {
    pub(crate) DataRedirectService {
        components = [Redirect, GlobalStateImpl, RepositoryImpl, FactoryImpl, CallbackCallerImpl],
        providers = []
    }
}

module! {
    pub(crate) EventService {
        components = [EventReceiveImpl, GlobalStateImpl, RepositoryImpl],
        providers = []
    }
}