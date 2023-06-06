pub trait StratumHandler: StratumProtocol {
    type CompatibleHeader: BlockHeader;
    type CompatibleClient: HeaderFetcher<HeaderT = Self::CompatibleHeader>;
    
    fn new(job_manager: JobManager<Self::CompatibleClient>) -> Self;
    fn process_request(
        &mut self,
        request: Self::Request,
    ) -> Self::Response;
}