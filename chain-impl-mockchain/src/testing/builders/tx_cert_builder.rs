use crate::{
    certificate::{Certificate, CertificatePayload, PoolOwnersSigned, PoolSignature},
    fee::FeeAlgorithm,
    fragment::Fragment,
    header::HeaderId,
    key::EitherEd25519SecretKey,
    testing::{builders::make_witness, data::Wallet, ledger::TestLedger},
    transaction::{
        AccountBindingSignature, Payload, SetAuthData, SetIOs, SingleAccountBindingSignature,
        TxBuilder, TxBuilderState,
    },
    value::Value,
};

pub struct TestTxCertBuilder<'a> {
    test_ledger: &'a TestLedger,
}

impl<'a> TestTxCertBuilder<'a> {
    pub fn new(test_ledger: &'a TestLedger) -> Self {
        Self { test_ledger }
    }

    fn block0_hash(&self) -> &HeaderId {
        &self.test_ledger.block0_hash
    }

    fn fee(&self, certificate: &Certificate) -> Value {
        let linear_fee = self.test_ledger.fee();
        let payload: CertificatePayload = certificate.into();
        linear_fee.calculate(Some(payload.as_slice()), 1, 0)
    }

    fn set_initial_ios<P: Payload>(
        &self,
        builder: TxBuilderState<SetIOs<P>>,
        funder: &Wallet,
        cert: &Certificate,
    ) -> TxBuilderState<SetAuthData<P>> {
        //utxo not supported yet
        let input = funder.make_input_with_value(&self.fee(cert));
        let builder = builder.set_ios(&[input], &[]);
        let witness = make_witness(
            self.block0_hash(),
            &funder.as_account_data(),
            &builder.get_auth_data_for_witness().hash(),
        );
        builder.set_witnesses(&[witness])
    }

    fn fragment(
        &self,
        cert: &Certificate,
        keys: Vec<EitherEd25519SecretKey>,
        funder: &Wallet,
    ) -> Fragment {
        match cert {
            Certificate::StakeDelegation(s) => {
                let builder = self.set_initial_ios(TxBuilder::new().set_payload(s), &funder, cert);
                let signature =
                    AccountBindingSignature::new_single(&keys[0], &builder.get_auth_data());
                let tx = builder.set_payload_auth(&signature);
                Fragment::StakeDelegation(tx)
            }
            Certificate::PoolRegistration(s) => {
                let builder = self.set_initial_ios(TxBuilder::new().set_payload(s), &funder, cert);
                let signature = pool_owner_sign(&keys, &builder);
                let tx = builder.set_payload_auth(&signature);
                Fragment::PoolRegistration(tx)
            }
            Certificate::PoolRetirement(s) => {
                let builder = self.set_initial_ios(TxBuilder::new().set_payload(s), &funder, cert);
                let signature = pool_owner_sign(&keys, &builder);
                let tx = builder.set_payload_auth(&signature);
                Fragment::PoolRetirement(tx)
            }
            Certificate::PoolUpdate(s) => {
                let builder = self.set_initial_ios(TxBuilder::new().set_payload(s), &funder, cert);
                let signature = pool_owner_sign(&keys, &builder);
                let tx = builder.set_payload_auth(&signature);
                Fragment::PoolUpdate(tx)
            }
            Certificate::OwnerStakeDelegation(s) => {
                let builder = self.set_initial_ios(TxBuilder::new().set_payload(s), &funder, cert);
                let tx = builder.set_payload_auth(&());
                Fragment::OwnerStakeDelegation(tx)
            }
        }
    }

    pub fn make_transaction(self, signers: &[&Wallet], certificate: &Certificate) -> Fragment {
        self.make_transaction_different_signers(&signers[0], signers, certificate)
    }

    pub fn make_transaction_different_signers(
        self,
        funder: &Wallet,
        signers: &[&Wallet],
        certificate: &Certificate,
    ) -> Fragment {
        let keys = signers
            .iter()
            .cloned()
            .map(|owner| owner.private_key())
            .collect();
        self.fragment(certificate, keys, funder)
    }
}

pub fn pool_owner_sign<P: Payload>(
    keys: &[EitherEd25519SecretKey],
    builder: &TxBuilderState<SetAuthData<P>>,
) -> PoolSignature {
    let pool_owner = pool_owner_signed(keys, builder);
    PoolSignature::Owners(pool_owner)
}

pub fn pool_owner_signed<P: Payload>(
    keys: &[EitherEd25519SecretKey],
    builder: &TxBuilderState<SetAuthData<P>>,
) -> PoolOwnersSigned {
    let auth_data = builder.get_auth_data();
    let mut sigs = Vec::new();
    for (i, key) in keys.iter().enumerate() {
        let sig = SingleAccountBindingSignature::new(key, &auth_data);
        sigs.push((i as u8, sig))
    }
    PoolOwnersSigned { signatures: sigs }
}
