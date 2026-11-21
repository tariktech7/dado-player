import assert from 'node:assert/strict';
import test from 'node:test';
import { resumeHistory, type Item } from '../src/types';

test('resume selects the latest episode per series without changing stored history', () => {
  const row = (id: string, seriesId: string | undefined, kind: Item['kind'], at: number) => ({
    item: {id, seriesId, kind, name:'Title', category:'', logo:'', url:''}, position:60, duration:100, at,
  });
  const history = [row('first','series-a','episode',1), row('latest','series-a','episode',32),
    row('other','series-b','episode',25), row('movie',undefined,'vod',40)];
  assert.deepEqual(resumeHistory(history).map(({item}) => item.id), ['movie','latest','other']);
  assert.deepEqual(history.map(({item}) => item.id), ['first','latest','other','movie']);
});
