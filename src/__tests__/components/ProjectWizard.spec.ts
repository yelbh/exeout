import { describe, it, expect, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import ProjectWizard from '../../components/ProjectWizard.vue';

describe('ProjectWizard', () => {
  it('renders correctly', () => {
    const wrapper = mount(ProjectWizard);
    expect(wrapper.find('h2').text()).toBe('Nouveau Projet');
  });

  it('updates project name', async () => {
    const wrapper = mount(ProjectWizard);
    const input = wrapper.find('input[type="text"]');
    await input.setValue('My New App');
    expect((wrapper.vm as any).project.name).toBe('My New App');
  });
});
