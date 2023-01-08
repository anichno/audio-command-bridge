1. Setup Google Cloud
	1. Create project at https://console.cloud.google.com/
	2. Enabled APIs & Services -> + ENABLE APIS AND SERVICES (button)
	3. Search and add Google Drive API
	4. OAuth consent screen
		1. Create and add an Authorized Domain (Must be real domain, not IP address)
		2. Add scopes:
			1. /auth/userinfo
			2.  /auth/drive
		3. Add test users (at least yourself)
	5. Credentials -> + CREATE CREDENTIALS -> OAuth client ID
	6. Create OAuth client ID
		1. Application type: Web application
		2. Set a name
		3. Add Authorized JavaScript origin
		4. Add an Authorized redired URI