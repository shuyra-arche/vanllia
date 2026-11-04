#![no_std]
#![no_main]

use execution::boot::Report;

#[mold::entry]
fn main() -> Report<'static> {
    let boot = match system_server::boot::take(router_api::PLIC_CLASS) {
        Ok(boot) => boot,
        Err(error) => {
            return Report::note(error.code(), "system: failed");
        }
    };
    let deployment = match unit::select_deployment(
        composition::DEPLOYMENTS,
        composition::PROGRAMS,
        boot.catalog.deployment(),
        boot.catalog.entry(),
    ) {
        Ok(deployment) => deployment,
        Err(why) => {
            system_support::debug!(
                "system: invalid deployment {}: {why:?}",
                boot.catalog.deployment()
            );
            return Report::note(system_server::E_PROGRAM, "system: failed");
        }
    };
    match system_server::run(boot, composition::PROGRAMS, deployment) {
        Ok(()) => Report::note(env::EXIT_OK, "system: done"),
        Err(reason) => Report::note(reason, "system: failed"),
    }
}
