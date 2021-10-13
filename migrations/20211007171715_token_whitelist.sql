CREATE TABLE token_whitelist
(
    address     VARCHAR NOT NULL,
    name        VARCHAR(255) NOT NULL,
    CONSTRAINT token_whitelist_pk PRIMARY KEY (address)
);

INSERT INTO token_whitelist (name, address) VALUES ('WTON',             '0:0ee39330eddb680ce731cd6a443c71d9069db06d149a9bec9569d1eb8d04eb37');
INSERT INTO token_whitelist (name, address) VALUES ('USDT',             '0:751b6e22687891bdc1706c8d91bf77281237f7453d27dc3106c640ec165a2abf');
INSERT INTO token_whitelist (name, address) VALUES ('USDC',             '0:1ad0575f0f98f87a07ec505c39839cb9766c70a11dadbfc171f59b2818759819');
INSERT INTO token_whitelist (name, address) VALUES ('DAI',              '0:95934aa6a66cb3eb211a80e99234dfbba6329cfa31600ce3c2b070d8d9677cef');
INSERT INTO token_whitelist (name, address) VALUES ('WBTC',             '0:6e76bccb41be2210dc9d7a4d0f3cbf0d5da592d0cb6b87662d5510f5b5efe497');
INSERT INTO token_whitelist (name, address) VALUES ('WETH',             '0:45f682b7e783283caef3f268e10073cf08842bce20041d5224c38d87df9f2e90');
INSERT INTO token_whitelist (name, address) VALUES ('UNI-V2-USDT-WTON', '0:53abe27ec16208973c9643911c35b5d033744fbb95b11b5672f71188db5a42dc');
INSERT INTO token_whitelist (name, address) VALUES ('BRIDGE',           '0:a453e9973010fadd4895e0d37c1ad15cba97f4fd31ef17663343f79768f678d9');
INSERT INTO token_whitelist (name, address) VALUES ('FRAX',             '0:f8b0314571f5f00f6d9a61a914b9b5e1d910442d09b80c27efeb46631d74e961');
INSERT INTO token_whitelist (name, address) VALUES ('FXS',              '0:0cddd021d2488c882041a31ba44e4ee69643a45223f068571e05b5a4c45bb7f6');
INSERT INTO token_whitelist (name, address) VALUES ('SUSHI',            '0:8d3c9d6e1803d1c3ee22130a08b370c075c99eca9f4eb6dffa1d5bcc34c45eac');
INSERT INTO token_whitelist (name, address) VALUES ('UNI',              '0:471c9d737254a0044695c7e50ec5b8f6f94eadd49511b298d4a331b95106652b');
INSERT INTO token_whitelist (name, address) VALUES ('AAVE',             '0:b2e341c01da068d43cfa0eae6dae36b12b476e55cf2c3eeb002689f44b9ddef9');
INSERT INTO token_whitelist (name, address) VALUES ('COMP',             '0:bc77ba7f3cbbebcca393e85ed479ef44df63cdee4fb572c3e0f904fb9fc63e25');
INSERT INTO token_whitelist (name, address) VALUES ('CRV',              '0:7dd7ae82835848dc6b490515ec4034968a8ceff893a6d5f31ab3cdfcfb79bbb6');
INSERT INTO token_whitelist (name, address) VALUES ('EURS',             '0:6b2baa777b89da66dddaf9f1602142987b13ca565bbb170da929ea945f5ce9fb');
INSERT INTO token_whitelist (name, address) VALUES ('TORN',             '0:387609364f765017fa3fa5815e08d420e054c88a86426cd6d5aaf2a1ee46ff5a');
INSERT INTO token_whitelist (name, address) VALUES ('YFI',              '0:e114f1f7d21ac6566d988c983315e0cdd5bee7b43c08918537d1117dea7e4534');
INSERT INTO token_whitelist (name, address) VALUES ('1INCH',            '0:3c66e3e0ce0a909ce8779b31509db773e544132d8fa6f6641c00bce257c79d9c');
INSERT INTO token_whitelist (name, address) VALUES ('DAF',              '0:bf1c7c0e8a187d9d5ba6069bf768b69a982df8b22ef8430b31dcc4f97263507e');
INSERT INTO token_whitelist (name, address) VALUES ('FRTN',             '0:7ffa7b7e72224a9a2fba27386dfa71ed379bd9d541662671ab096e22110e5e96');
INSERT INTO token_whitelist (name, address) VALUES ('EUPi',             '0:f4a912b0c3be422e02c0a8295590671cae5b38c75d397da8d1da33882888bbcb');
