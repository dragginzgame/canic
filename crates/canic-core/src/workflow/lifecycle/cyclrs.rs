// cycles_response
async fn cycles_response(req: &CyclesRequest) -> Result<Response, Error> {
    deposit_cycles(msg_caller(), req.cycles).await?;

    let cycles_transferred = req.cycles;

    Ok(Response::Cycles(CyclesResponse { cycles_transferred }))
}
