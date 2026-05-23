import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface FrontstageDesignModeState {
  isDesignMode: boolean;
  setDesignMode: (enabled: boolean) => void;
  toggleDesignMode: () => void;
}

const initialState = {
  isDesignMode: false
};

export const useFrontstageDesignModeStore = create<FrontstageDesignModeState>(
  persist(
    (set) => ({
      ...initialState,
      setDesignMode: (enabled) => set({ isDesignMode: enabled }),
      toggleDesignMode: () =>
        set((state) => ({ isDesignMode: !state.isDesignMode }))
    }),
    {
      name: 'frontstage-design-mode'
    }
  )
);

export function resetFrontstageDesignModeStore() {
  useFrontstageDesignModeStore.setState(initialState);
  useFrontstageDesignModeStore.persist?.clearStorage();
}
