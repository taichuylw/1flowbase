import { create } from 'zustand';

interface FrontstageDesignModeState {
  isDesignMode: boolean;
  setDesignMode: (enabled: boolean) => void;
  toggleDesignMode: () => void;
}

const initialState = {
  isDesignMode: false
};

export const useFrontstageDesignModeStore = create<FrontstageDesignModeState>(
  (set) => ({
    ...initialState,
    setDesignMode: (enabled) => set({ isDesignMode: enabled }),
    toggleDesignMode: () =>
      set((state) => ({ isDesignMode: !state.isDesignMode }))
  })
);

export function resetFrontstageDesignModeStore() {
  useFrontstageDesignModeStore.setState(initialState);
}
