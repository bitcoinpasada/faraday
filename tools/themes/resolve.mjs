// Faraday's, not the reference's (docs/THEMES.md §10.1): a resolve hook
// that adds `.js` to a relative import written without it. Ten of
// material-color-utilities' published files import a sibling that way,
// written for bundlers, and Node's resolver refuses it. The vendored
// files stay byte for byte as published.
import { registerHooks } from 'node:module';

registerHooks({
  resolve(specifier, context, next) {
    if (specifier.startsWith('.') && !/\.[cm]?js$/.test(specifier)) {
      return next(specifier + '.js', context);
    }
    return next(specifier, context);
  },
});
