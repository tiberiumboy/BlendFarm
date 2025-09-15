default:
	cd ./src-tauri/ 
	cargo tauri dev
	# what can we do afterward?

# could be renamed to release?
build:
	cd ./src-tauri/
	cargo tauri build
	# maybe a command to bundle a release and upload gpg keys / etc?

rebuild_database: .sqlx
	cd ./src-tauri/			# navigate to Tauri's codebase
	cargo sqlx db create	# create the database file
	cargo sqlx mig run		# invoke all sql up table files inside ./migrations/ folder
	cargo sqlx prepare		# create cache sql result that satisfy cargo compiler

test:
	cd ./src-tauri/ && cargo test

clean:
	rm -rf ./src-tauri/target ./src-tauri/