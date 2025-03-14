use super::*;

impl_orm_object!(
    ComputeResourceGroup,
    "compute_resource_group",
    cost_per_cpu_hour: BigDecimal,
    cost_per_gpu_hour: BigDecimal,
    cost_per_gb_hour: BigDecimal,
    cost_per_net_rx_gb: BigDecimal,
    cost_per_net_tx_gb: BigDecimal,
    deployment_base_url: Option<String>
);
