import 'babel-polyfill'
import 'dotenv/config'
import Koa from 'koa'
import Raven from 'raven'

import compress from 'koa-compress'
import cors from '@koa/cors'
import mount from 'koa-mount'
import send from 'koa-send'
import serve from 'koa-static'

import api from './api'
import proxy from './proxy'

// Set up Sentry
if (process.env.NODE_ENV === 'production' && process.env.RAVEN_DSN) {
	Raven.config(process.env.RAVEN_DSN).install()
}

// Public url to serve SPA stuff from
const publicPath = process.env.PUBLIC_PATH || 'public'

const app = new Koa()

// Shrink that stuff up a bit
app.use(compress())

// Loosen up CORS a bit
app.use(cors())

// Serve the main public assets
app.use(serve(publicPath))

// Mount the fancy stuff
app.use(mount('/api', api))
app.use(mount('/proxy/fflogs', proxy))

// All other urls should serve up the index file, routing is handled by the SPA
app.use(async ctx => {
	// Don't do the fallback for files in /static, though - that just causes syntax errors when things go wrong
	if (ctx.url.startsWith('/static')) {
		return
	}
	await send(ctx, 'index.html', {root: publicPath})
})

// Boot the server
app.listen(process.env.PORT || 3001)
