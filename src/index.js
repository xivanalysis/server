import '@babel/polyfill'
import 'dotenv/config'
import fs from 'fs'
import Koa from 'koa'
import path from 'path'
import Raven from 'raven'

import compress from 'koa-compress'
import cors from '@koa/cors'
import mount from 'koa-mount'
import send from 'koa-send'
import serve from 'koa-static'

import api from './api'
import proxy from './proxy'

// Defaults
const DEFAULT_PORT = 3001

// Set up Sentry
if (process.env.NODE_ENV === 'production' && process.env.RAVEN_DSN) {
	Raven.config(process.env.RAVEN_DSN).install()
}

// Public url to serve SPA stuff from
const publicPath = process.env.PUBLIC_PATH || 'public'
const defaultClientBranch = process.env.DEFAULT_CLIENT_BRANCH || 'master'

const app = new Koa()

// Shrink that stuff up a bit
app.use(compress())

// Loosen up CORS a bit
app.use(cors())

// Work out the final public path
async function canRead(path) {
	return new Promise(resolve => {
		fs.access(path, fs.constants.R_OK, err => resolve(!err))
	})
}
app.use(async (ctx, next) => {
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
})

// Serve the main public assets
app.use(async (ctx, ...rest) => serve(ctx.state.assetPath)(ctx, ...rest))

// Mount the fancy stuff
app.use(mount('/api', api))
app.use(mount('/proxy/fflogs', proxy))

// All other urls should serve up the index file, routing is handled by the SPA
app.use(async ctx => {
	// Don't do the fallback for files in /assets, though - that just causes syntax errors when things go wrong
	if (ctx.url.startsWith('/assets')) {
		return
	}
	await send(ctx, 'index.html', {root: ctx.state.assetPath})
})

// Boot the server
app.listen(process.env.PORT || DEFAULT_PORT)
