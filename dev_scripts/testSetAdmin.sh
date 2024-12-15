dfx identity use eden
dfx canister call eden_vault_backend create_new_user '(
	principal "ogxac-f4uay-l5nc4-dth55-ubkk6-ulcjz-3i643-y2hwa-3cicp-gtbur-qae",
	vec {1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1}
)'
dfx identity use default