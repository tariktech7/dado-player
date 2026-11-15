import { test } from 'node:test';
import assert from 'node:assert/strict';
import { languageKey } from '../src/languages';

test('provider language aliases match preferred languages across regions', () => {
  for (const value of ['de', 'ger', 'DEU-de', ' de_AT ']) assert.equal(languageKey(value), 'de');
  assert.equal(languageKey('ENG-us'), 'en');
  assert.equal(languageKey('fre'), 'fr');
  assert.equal(languageKey('tur'), 'tr');
  assert.equal(languageKey(''), '');
  assert.equal(languageKey('und'), 'und');
});
