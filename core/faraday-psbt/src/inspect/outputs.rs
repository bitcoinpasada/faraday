//! What inspection reads from each output: where it pays, what kind it
//! is, whether it is dust, and what the outputs add up to.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use bitcoin::{Address, Amount, Script, VarInt};

use super::inputs::Walk;
use super::{
    Detail, Level, OutputInfo, OutputKind, OwnWallet, Warning, WarningKind, change_or_transfer,
    output_kind, warn,
};

/// What the outputs add up to.
pub(super) struct Outputs {
    pub outputs: Vec<OutputInfo>,
    pub total_out: Amount,
    pub amount_to_others: Amount,
    pub change_total: Amount,
}

/// Costs what `output_kind` costs for each output. Reads every output:
/// its address, kind and dust, adding its weight to `weight`.
pub(super) fn read_outputs(
    w: &Walk,
    spent: &[OwnWallet],
    warnings: &mut Vec<Warning>,
    weight: &mut usize,
) -> Outputs {
    let inner = w.psbt.inner();
    let tx = &inner.unsigned_tx;
    let network = bitcoin::Network::from(w.ctx.network);
    let mut outputs = Vec::with_capacity(tx.output.len());
    let mut total_out = Amount::ZERO;
    let mut amount_to_others = Amount::ZERO;
    let mut change_total = Amount::ZERO;
    for (i, txout) in tx.output.iter().enumerate() {
        let out = &inner.outputs[i];
        let spk = &txout.script_pubkey;
        let script_len = spk.len();
        *weight += (8 + VarInt::from(script_len).size() + script_len) * 4;
        total_out = total_out
            .checked_add(txout.value)
            .unwrap_or(Amount::MAX_MONEY);

        let address = output_address(spk, network, i, warnings);

        let (kind, own) = output_kind(w.c, w.secp, out, txout, w.ctx, i, warnings);
        let kind = change_or_transfer(kind, own, spent, w.ctx, i, warnings);
        match kind {
            OutputKind::Change { .. } | OutputKind::WalletChange { .. } => {
                change_total = change_total
                    .checked_add(txout.value)
                    .unwrap_or(Amount::MAX_MONEY);
            }
            _ => {
                amount_to_others = amount_to_others
                    .checked_add(txout.value)
                    .unwrap_or(Amount::MAX_MONEY);
            }
        }

        let is_dust = txout.value < spk.minimal_non_dust();
        if is_dust {
            warn(
                warnings,
                Level::Caution,
                WarningKind::DustOutput,
                Detail::Dust {
                    output: i,
                    sat: txout.value.to_sat(),
                },
            );
        }

        outputs.push(OutputInfo {
            index: i,
            address,
            script_pubkey: spk.clone(),
            amount: txout.value,
            kind,
            is_dust,
        });
    }
    Outputs {
        outputs,
        total_out,
        amount_to_others,
        change_total,
    }
}

/// The address output `i` pays; a script with none is a data output or a
/// non-standard one, said so, and its address is empty.
fn output_address(
    spk: &Script,
    network: bitcoin::Network,
    i: usize,
    warnings: &mut Vec<Warning>,
) -> String {
    match Address::from_script(spk, network) {
        Ok(a) => a.to_string(),
        Err(_) => {
            let (level, detail) = if spk.is_op_return() {
                (Level::Info, Detail::DataOutput { output: i })
            } else {
                (Level::Caution, Detail::NonStandardOutput { output: i })
            };
            warn(warnings, level, WarningKind::NonStandardScript, detail);
            String::new()
        }
    }
}
