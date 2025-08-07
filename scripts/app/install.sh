#!/bin/bash

dfx canister create --all -qq
dfx build --all

dfx ledger fabricate-cycles --canister root --cycles 9000000000000000
dfx canister install root --mode=reinstall -y