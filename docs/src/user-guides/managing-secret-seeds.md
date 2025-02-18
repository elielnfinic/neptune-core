# Managing Secret Seeds

The wallet derives all spending keys and receiving addresses from a secret seed. It does this *deterministically*, meaning that with a back-up of the secret seed you can re-derive the exact same keys and addresses. Moreover, with the exception of off-chain UTXO notifications, all incoming payments have on-chain ciphertexts that (when decrypted) provide all necessary information to spend the funds at a later date. Put together, this construction means that (with the exception of payments with off-chain UTXO notifications):

 > a back-up of the secret seed, along with historical blockchain data, suffices to reclaim all funds.

## Wallet File

By default, `neptune-core` stores the wallet secret seed to and reads it from `[data-dir]/neptune/[network]/wallet/wallet.dat`. Here `[data-dir]` is the data directory and this directory is the second line in the log output when running `neptune-core`. The `[network]` is `main` unless you are not on mainnet.

A convenient command is `> neptune-cli which-wallet`, which shows the location of the wallet file.

> **Warning:** do not share your wallet file with other people, especially not other people claiming to help you.

## Incoming Sender Randomness

There is another file in the same data directory called `incoming_randomness.dat`. It contains data that you *also* need to spend funds, but since this data is generated by the sender and not the receiver, it cannot be derived from the wallet's secret seed.

The incoming sender randomness is always part of the information payload sent (in encrypted form) to the beneficiary of a transaction, along with the amount of funds transferred. With the exception of off-chain UTXO transfers, this ciphertext lives on the blockchain; and so with that exception, the blockchain serves to back up the information in `incoming_randomness.dat`.

If you do receive transactions with off-chain UTXO notifications, it is recommended to either a) back up this file or b) consolidate your funds by sending them to yourself via a transaction with on-chain notification.

## New Secret Seed

By default, `neptune-core` will read the wallet file. If none exists, it will generate one and populate it with a random secret seed.

To generate a new secret seed and `wallet.dat` without starting `neptune-core`, use the CLI: `> neptune-cli generate-wallet`.

Note that this command does nothing if the wallet file already exists. If you want to invoke this command even though a `wallet.dat` file already exists, rename it first.

## Secret Seed Phrase

Neptune supports [BIP-39](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki) secret seed phrases. A secret seed phrase consists of 18 simple English words, such as the ones shown below. Secret seeds can be exported to phrases and vice versa; the point is that the phrase is easier to back up, for instance by physically carving it into fire-proof stone.

```
1. toilet
2. trick
3. shiver
4. never
5. can
6. frown
7. gonna
8. mirror
9. mail
10. let
11. connect
12. oven
13. you
14. type
15. pill
16. down
17. vast
18. view
```

 - To export a seed phrase: `> neptune-cli export-seed-phrase`. This command will read from the `wallet.dat` file and will fail if that file does not exist.
 - To import a seed phrase: `> neptune-cli import-seed-phrase`. Note that this command will not do anything if a `wallet.dat` file already exists.
