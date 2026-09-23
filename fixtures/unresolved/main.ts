import './missing';
import '@/alias';
import '../outside';
import React from 'react';
import { thing } from '@scope/package';
import { value } from './value.js';
export * from './lib';
const lazy = import(moduleName);
