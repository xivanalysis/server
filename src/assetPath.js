import fs from 'fs'
import path from 'path'

// Public url to serve SPA stuff from
const publicPath = process.env.PUBLIC_PATH || 'public'
const defaultClientBranch = process.env.DEFAULT_CLIENT_BRANCH || 'master'

async function canRead(path) {
	return new Promise(resolve => {
		fs.access(path, fs.constants.R_OK, err => resolve(!err))
	})
}

const assetPathMiddleware = () => async (ctx, next) => {
	// Default to the legacy everything-in-top-level structure
	let assetPath = publicPath

	if (ctx.subdomains.length) {
		// If there's subdomains, use the foremost (which is last in the array :eyes:)
		const subdomain = ctx.subdomains[ctx.subdomains.length - 1]
		assetPath = path.join(publicPath, subdomain)
		// If we can't find the files for it, kill the request
		if (!(await canRead(assetPath))) {
			return
		}
	} else {
		// Otherwise, try to use the default client branch
		const newPath = path.join(publicPath, defaultClientBranch)
		if (await canRead(newPath)) {
			assetPath = newPath
		}
	}

	ctx.state.assetPath = assetPath
	await next()
}

export default assetPathMiddleware
