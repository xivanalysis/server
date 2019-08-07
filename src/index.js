import 'core-js/stable'
import 'regenerator-runtime/runtime'
import 'dotenv/config'
import Koa from 'koa'
import Raven from 'raven'

import compress from 'koa-compress'
import cors from '@koa/cors'
import mount from 'koa-mount'
import send from 'koa-send'
import serve from 'koa-static'

import assetPath from './assetPath'
import fflogsProxy from './fflogsProxy'
import xivapi from './xivapi'

// Defaults
const DEFAULT_PORT = 3001

// Set up Sentry
if (process.env.NODE_ENV === 'production' && process.env.RAVEN_DSN) {
	Raven.config(process.env.RAVEN_DSN).install()
}

const app = new Koa()

// Shrink that stuff up a bit
app.use(compress())

// Loosen up CORS a bit
app.use(cors())

// Work out the final public path
app.use(assetPath())

// Serve the main public assets
app.use(async (ctx, ...rest) => serve(ctx.state.assetPath)(ctx, ...rest))

// Mount the fancy stuff
app.use(mount('/proxy/fflogs', fflogsProxy))
app.use(mount('/xivapi', xivapi))

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
