ALTER TABLE token_whitelist ADD COLUMN version twa_token_wallet_version NOT NULL DEFAULT 'OldTip3v4';

INSERT INTO token_whitelist (name, address, version) VALUES ('WEVER',            '0:a49cd4e158a9a15555e624759e2e4e766d22600b7800d891e46f9291f044a93d', 'Tip3');
INSERT INTO token_whitelist (name, address, version) VALUES ('QUBE',             '0:9f20666ce123602fd7a995508aeaa0ece4f92133503c0dfbd609b3239f3901e2', 'Tip3');
INSERT INTO token_whitelist (name, address, version) VALUES ('USDT',             '0:a519f99bb5d6d51ef958ed24d337ad75a1c770885dcd42d51d6663f9fcdacfb2', 'Tip3');
INSERT INTO token_whitelist (name, address, version) VALUES ('USDC',             '0:c37b3fafca5bf7d3704b081fde7df54f298736ee059bf6d32fac25f5e6085bf6', 'Tip3');
INSERT INTO token_whitelist (name, address, version) VALUES ('DAI',              '0:eb2ccad2020d9af9cec137d3146dde067039965c13a27d97293c931dae22b2b9', 'Tip3');
INSERT INTO token_whitelist (name, address, version) VALUES ('WBTC',             '0:2ba32b75870d572e255809b7b423f30f36dd5dea075bd5f026863fceb81f2bcf', 'Tip3');
INSERT INTO token_whitelist (name, address, version) VALUES ('WETH',             '0:59b6b64ac6798aacf385ae9910008a525a84fc6dcf9f942ae81f8e8485fe160d', 'Tip3');
INSERT INTO token_whitelist (name, address, version) VALUES ('UNI-V2-USDT-WTON', '0:1e6e1b3674b54753864af7b15072566ce632965bd83bab431a8ff86d68cf1657', 'Tip3');
INSERT INTO token_whitelist (name, address, version) VALUES ('BRIDGE',           '0:f2679d80b682974e065e03bf42bbee285ce7c587eb153b41d761ebfd954c45e1', 'Tip3');
INSERT INTO token_whitelist (name, address, version) VALUES ('FRAX',             '0:efed9f9a7e6c455ee60829fd003b2f42edda513c6f19a484f916b055e9aa58d2', 'Tip3');
INSERT INTO token_whitelist (name, address, version) VALUES ('FXS',              '0:c14e2f026feaae0f99b92c04ee421051a782fff60156ac8a586a12f63d7facef', 'Tip3');
INSERT INTO token_whitelist (name, address, version) VALUES ('EURS',             '0:00ca16398f314a9b3bed582dc69582515d866ededb6c4e18190f63b305cedf91', 'Tip3');
INSERT INTO token_whitelist (name, address, version) VALUES ('DAF',              '0:f48054939064d686a9ad68d96d9ab79e409b095557c06ab7f073097dade7057f', 'Tip3');
