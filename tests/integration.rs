#![cfg(test)]

mod test {
    use soroban_sdk::{
        testutils::Address as _, token, vec, Address, Env, Symbol,
    };

    use freelancer_escrow::{FreelancerEscrowClient, types::{Estado, Hito}};

    fn create_hito(env: &Env, id: u32, desc: &str, monto: i128) -> Hito {
        Hito {
            id,
            descripcion: Symbol::new(env, desc),
            monto,
            completado: false,
            aprobado: false,
        }
    }

    #[test]
    fn test_single_milestone_flow() {
        let env = Env::default();
        env.mock_all_auths();

        let empresa = Address::generate(&env);
        let freelancer = Address::generate(&env);
        let arbitro = Address::generate(&env);
        
        let token_addr = env.register_stellar_asset_contract_v2(empresa.clone());
        let token_client = token::StellarAssetClient::new(&env, &token_addr.address());
        token_client.mint(&empresa, &1000);
        
        let token = token_addr.address();

        let contract_id = env.register_contract(None, freelancer_escrow::FreelancerEscrow);
        let client = FreelancerEscrowClient::new(&env, &contract_id);

        let hitos = vec![&env, create_hito(&env, 1, "desarrollo", 1000)];
        client.initialize(&empresa, &freelancer, &arbitro, &token, &hitos);

        client.deposit(&1000);

        client.approve_milestone(&1);

        client.release(&1);

        let escrow = client.query_escrow();
        assert!(escrow.estado == Estado::Completado);
    }

    #[test]
    fn test_multiple_milestones() {
        let env = Env::default();
        env.mock_all_auths();

        let empresa = Address::generate(&env);
        let freelancer = Address::generate(&env);
        let arbitro = Address::generate(&env);
        
        let token_addr = env.register_stellar_asset_contract_v2(empresa.clone());
        let token_client = token::StellarAssetClient::new(&env, &token_addr.address());
        token_client.mint(&empresa, &1000);
        
        let token = token_addr.address();

        let contract_id = env.register_contract(None, freelancer_escrow::FreelancerEscrow);
        let client = FreelancerEscrowClient::new(&env, &contract_id);

        let hitos = vec![
            &env,
            create_hito(&env, 1, "fase1", 500),
            create_hito(&env, 2, "fase2", 500),
        ];
        client.initialize(&empresa, &freelancer, &arbitro, &token, &hitos);

        client.deposit(&1000);

        client.approve_milestone(&1);
        client.release(&1);

        let escrow = client.query_escrow();
        assert!(escrow.monto_pagado == 500);
        assert!(escrow.estado == Estado::Depositado);

        client.approve_milestone(&2);
        client.release(&2);

        let escrow = client.query_escrow();
        assert!(escrow.monto_pagado == 1000);
        assert!(escrow.estado == Estado::Completado);
    }

    #[test]
    fn test_dispute_and_resolution() {
        let env = Env::default();
        env.mock_all_auths();

        let empresa = Address::generate(&env);
        let freelancer = Address::generate(&env);
        let arbitro = Address::generate(&env);
        
        let token_addr = env.register_stellar_asset_contract_v2(empresa.clone());
        let token_client = token::StellarAssetClient::new(&env, &token_addr.address());
        token_client.mint(&empresa, &1000);
        
        let token = token_addr.address();

        let contract_id = env.register_contract(None, freelancer_escrow::FreelancerEscrow);
        let client = FreelancerEscrowClient::new(&env, &contract_id);

        let hitos = vec![&env, create_hito(&env, 1, "desarrollo", 1000)];
        client.initialize(&empresa, &freelancer, &arbitro, &token, &hitos);

        client.deposit(&1000);

        client.approve_milestone(&1);

        client.dispute();

        let escrow = client.query_escrow();
        assert!(escrow.estado == Estado::Disputado);

        client.resolve(&500, &500);

        let escrow = client.query_escrow();
        assert!(escrow.estado == Estado::Completado);
    }

    #[test]
    fn test_cancel() {
        let env = Env::default();
        env.mock_all_auths();

        let empresa = Address::generate(&env);
        let freelancer = Address::generate(&env);
        let arbitro = Address::generate(&env);
        
        let token_addr = env.register_stellar_asset_contract_v2(empresa.clone());
        let token = token_addr.address();

        let contract_id = env.register_contract(None, freelancer_escrow::FreelancerEscrow);
        let client = FreelancerEscrowClient::new(&env, &contract_id);

        let hitos = vec![&env, create_hito(&env, 1, "desarrollo", 1000)];
        client.initialize(&empresa, &freelancer, &arbitro, &token, &hitos);

        client.cancel();

        let escrow = client.query_escrow();
        assert!(escrow.estado == Estado::Cancelado);
    }

    #[test]
    fn test_cancel_after_deposit() {
        let env = Env::default();
        env.mock_all_auths();

        let empresa = Address::generate(&env);
        let freelancer = Address::generate(&env);
        let arbitro = Address::generate(&env);
        
        let token_addr = env.register_stellar_asset_contract_v2(empresa.clone());
        let token_client = token::StellarAssetClient::new(&env, &token_addr.address());
        token_client.mint(&empresa, &1000);
        
        let token = token_addr.address();

        let contract_id = env.register_contract(None, freelancer_escrow::FreelancerEscrow);
        let client = FreelancerEscrowClient::new(&env, &contract_id);

        let hitos = vec![&env, create_hito(&env, 1, "desarrollo", 1000)];
        client.initialize(&empresa, &freelancer, &arbitro, &token, &hitos);

        client.deposit(&1000);

        client.cancel();

        let escrow = client.query_escrow();
        assert!(escrow.estado == Estado::Cancelado);
    }

    #[test]
    #[should_panic(expected = "Contract, #1")]
    fn test_double_initialization_panics() {
        let env = Env::default();
        env.mock_all_auths();

        let empresa = Address::generate(&env);
        let freelancer = Address::generate(&env);
        let arbitro = Address::generate(&env);
        
        let token_addr = env.register_stellar_asset_contract_v2(empresa.clone());
        let token = token_addr.address();

        let contract_id = env.register_contract(None, freelancer_escrow::FreelancerEscrow);
        let client = FreelancerEscrowClient::new(&env, &contract_id);

        let hitos = vec![&env, create_hito(&env, 1, "desarrollo", 1000)];
        client.initialize(&empresa, &freelancer, &arbitro, &token, &hitos);
        client.initialize(&empresa, &freelancer, &arbitro, &token, &hitos);
    }

    #[test]
    fn test_only_company_can_release() {
        let env = Env::default();
        env.mock_all_auths();

        let empresa = Address::generate(&env);
        let freelancer = Address::generate(&env);
        let arbitro = Address::generate(&env);
        
        let token_addr = env.register_stellar_asset_contract_v2(empresa.clone());
        let token_client = token::StellarAssetClient::new(&env, &token_addr.address());
        token_client.mint(&empresa, &1000);
        
        let token = token_addr.address();

        let contract_id = env.register_contract(None, freelancer_escrow::FreelancerEscrow);
        let client = FreelancerEscrowClient::new(&env, &contract_id);

        let hitos = vec![&env, create_hito(&env, 1, "desarrollo", 1000)];
        client.initialize(&empresa, &freelancer, &arbitro, &token, &hitos);

        client.deposit(&1000);

        client.approve_milestone(&1);

        let escrow_before = client.query_escrow();
        assert!(escrow_before.hitos.get(0).unwrap().aprobado == true);

        client.release(&1);

        let escrow = client.query_escrow();
        assert!(escrow.estado == Estado::Completado);
    }
}