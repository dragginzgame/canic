#!/bin/bash

dfx canister create test -qq
dfx build test
dfx canister create root -qq
dfx build root


dfx ledger fabricate-cycles --canister root --cycles 9000000000000000
dfx canister install root --mode=reinstall -y