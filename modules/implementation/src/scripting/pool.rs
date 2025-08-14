use std::sync::Arc;

struct Inner<T, C> {
    vms: Vec<Arc<super::UserVM<T, C>>>,
}

#[derive(Clone)]
pub struct Pool<T, C>(Arc<Inner<T, C>>);

impl<T, C> Pool<T, C> {
    pub fn get(&self) -> Arc<super::UserVM<T, C>> {
        let mut key: [u8; 1] = [0; 1];

        rand::fill(&mut key);

        self.0.vms[key[0] as usize % self.0.vms.len()].clone()
    }
}

pub async fn new<T, C, F>(cnt: usize, vm_supplier: impl Fn() -> F) -> anyhow::Result<Pool<T, C>>
where
    F: std::future::Future<Output = anyhow::Result<super::UserVM<T, C>>>,
{
    let mut vms = Vec::new();
    for _i in 0..cnt {
        vms.push(Arc::new(vm_supplier().await?));
    }

    let inner = Inner { vms };

    Ok(Pool(Arc::new(inner)))
}
